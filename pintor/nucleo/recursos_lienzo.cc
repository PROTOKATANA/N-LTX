# include "recursos_lienzo.h"

# include <algorithm>

# include <cstdio>

# include <cstdlib>

# include <string>

# include <unordered_set>

# include "sombreadores_horneados.h"

# include "sombreadores_lienzo.h"

using namespace Diligent ;

namespace ltx {

namespace {

bool fallo(std :: string & error , const char * mensaje) {

	error = mensaje ;

	return false ;

}

// Crea o agranda un búfer, con capacidad de sobra para no hacerlo cada vez

bool asegurar_buffer(

	const Gpu & gpu ,

	RefCntAutoPtr<IBuffer> & buffer ,

	std :: uint64_t & capacidad ,

	std :: uint64_t necesario ,

	BIND_FLAGS flags ,

	const char * nombre ,

	std :: string & error

) {

	if (buffer && capacidad >= necesario) { return true ; }

	const std :: uint64_t nueva = std :: max<std :: uint64_t>(necesario * 3 / 2 , 1 << 16) ;

	BufferDesc desc ;

	desc.Name = nombre ;

	desc.Size = nueva ;

	desc.Usage = USAGE_DEFAULT ;

	desc.BindFlags = flags ;

	buffer.Release() ;

	desc.ImmediateContextMask = gpu.mascara() ;

	gpu.device->CreateBuffer(desc , nullptr , &buffer) ;

	if (!buffer) { return fallo(error , "OUT_OF_GPU_MEMORY: buffer de GPU") ; }

	capacidad = nueva ;

	return true ;

}

constexpr BlendStateDesc premultiplicada() {

	BlendStateDesc mezcla ;

	auto & rt = mezcla.RenderTargets[0] ;

	rt.BlendEnable = True ;

	rt.SrcBlend = BLEND_FACTOR_ONE ;

	rt.DestBlend = BLEND_FACTOR_INV_SRC_ALPHA ;

	rt.SrcBlendAlpha = BLEND_FACTOR_ONE ;

	rt.DestBlendAlpha = BLEND_FACTOR_INV_SRC_ALPHA ;

	return mezcla ;

}

// Los tipos de tinta, como los dice Rust

std :: string con_tipos(const char * fuente) {

	const auto definir = [](const char * nombre , ltx :: TipoTinta tipo) {

		return std :: string("#define ") + nombre + " " + std :: to_string(static_cast<std :: uint32_t>(tipo)) + "\n" ;

	} ;

	return

		definir("TIPO_GLIFO" , ltx :: TipoTinta :: Glifo) +

		definir("TIPO_LINEAL" , ltx :: TipoTinta :: Lineal) +

		definir("TIPO_RADIAL" , ltx :: TipoTinta :: Radial) +

		definir("TIPO_TRAZO" , ltx :: TipoTinta :: Trazo) +

		fuente ;

}

// FNV-1a de 64 bits : la clave de lo horneado es el texto del shader

std :: uint64_t huella(const char * fuente) {

	std :: uint64_t h = 0xcbf29ce484222325ull ;

	for (; * fuente ; fuente++) { h = (h ^ static_cast<unsigned char>(* fuente)) * 0x100000001b3ull ; }

	return h ;

}

const Horneado * horneado(const char * fuente) {

	const std :: uint64_t buscada = huella(fuente) ;

	for (const Horneado & h : horneados) {

		if (h.huella == buscada) { return &h ; }

	}

	return nullptr ;

}

ShaderCreateInfo descripcion(const char * nombre , SHADER_TYPE tipo) {

	ShaderCreateInfo info ;

	info.SourceLanguage = SHADER_SOURCE_LANGUAGE_HLSL ;

	info.Desc.UseCombinedTextureSamplers = true ;

	info.EntryPoint = "main" ;

	info.Desc.Name = nombre ;

	info.Desc.ShaderType = tipo ;

	return info ;

}

// Compilar HLSL en cada arranque se llevaba 32 de los 36 ms de preparar el pintor, y no depende de
// ningún documento : en Vulkan se usa lo horneado. Si el texto cambia, su huella ya no está y se
// compila como antes. Otros backends compilan siempre

RefCntAutoPtr<IShader> compilar(IRenderDevice * device , const char * nombre , SHADER_TYPE tipo , const char * fuente) {

	ShaderCreateInfo info = descripcion(nombre , tipo) ;

	const Horneado * hecho = device->GetDeviceInfo().IsVulkanDevice() ? horneado(fuente) : nullptr ;

	if (hecho) {

		info.ByteCode = hecho->palabras ;

		info.ByteCodeSize = hecho->cuantas * sizeof(std :: uint32_t) ;

	} else {

		info.Source = fuente ;

	}

	RefCntAutoPtr<IShader> shader ;

	device->CreateShader(info , &shader) ;

	return shader ;

}

void fijar_estatica(IPipelineState * pipeline , SHADER_TYPE etapa , const char * nombre , IDeviceObject * objeto) {

	if (auto * variable = pipeline->GetStaticVariableByName(etapa , nombre)) { variable->Set(objeto) ; }

}

// Los trazos y los glifos no dependen del MSAA : solo los rellenos, y con 8× sus
// curvas pierden los dientes. Si la GPU no llega, lo que tenga

std :: uint32_t muestras_posibles(IRenderDevice * device) {

	const char * pedidas = std :: getenv("NLTX_MSAA") ;

	const std :: uint32_t n = pedidas ? static_cast<std :: uint32_t>(std :: atoi(pedidas)) : 8u ;

	const auto cuentas = device->GetTextureFormatInfoExt(formato_color).SampleCounts ;

	for (std :: uint32_t m = std :: min(n , 8u) ; m > 1 ; m /= 2) {

		if (cuentas & m) { return m ; }

	}

	return 2 ;

}

// Solo hace falta el stencil, y D24 ocupa la mitad, pero no todas las GPU lo tienen

TEXTURE_FORMAT elegir_plantilla(IRenderDevice * device , std :: uint32_t muestras) {

	const auto & info = device->GetTextureFormatInfoExt(TEX_FORMAT_D24_UNORM_S8_UINT) ;

	const bool sirve = info.Supported && (info.BindFlags & BIND_DEPTH_STENCIL) && (info.SampleCounts & muestras) ;

	return sirve ? TEX_FORMAT_D24_UNORM_S8_UINT : TEX_FORMAT_D32_FLOAT_S8X24_UINT ;

}

// Una pasada que pinta triángulos de una lista

struct RecetaLista {

	const char * nombre ;

	IShader * vs ;

	IShader * ps ;

	bool pinta ;

	STENCIL_OP paso ;

	// La textura propia que cambia en cada dibujo, si la lleva

	const char * textura ;

} ;

// Una pasada que pinta la pantalla entera con lo que hay en `Capa`, y una
// segunda textura si hace falta

struct RecetaCapa {

	const char * nombre ;

	IShader * vs ;

	IShader * ps ;

	const char * segunda ;

	bool mezclada ;

	// a la superficie de quien integra : una muestra y sin stencil

	bool final_ ;

} ;

}  // namespace

bool Objetivo :: crear(const Gpu & gpu , const FormatoObjetivo & formato , std :: string & error) {

	TextureDesc desc ;

	desc.Name = "LTX objetivo" ;

	desc.Type = RESOURCE_DIM_TEX_2D ;

	desc.Width = formato.ancho ;

	desc.Height = formato.alto ;

	desc.Format = formato_color ;

	desc.SampleCount = formato.muestras ;

	desc.BindFlags = BIND_RENDER_TARGET ;

	desc.ImmediateContextMask = gpu.mascara() ;

	gpu.device->CreateTexture(desc , nullptr , &color) ;

	desc.Format = formato.plantilla ;

	desc.BindFlags = BIND_DEPTH_STENCIL ;

	gpu.device->CreateTexture(desc , nullptr , &plantilla) ;

	desc.Format = formato_color ;

	desc.SampleCount = 1 ;

	desc.BindFlags = BIND_RENDER_TARGET | BIND_SHADER_RESOURCE ;

	gpu.device->CreateTexture(desc , nullptr , &resuelta) ;

	return (color && plantilla && resuelta) || fallo(error , "No se pudo crear un objetivo de LTX") ;

}

bool Malla :: subir(const Gpu & gpu , const Geometria & geometria , std :: string & error) {

	const auto & v = geometria.vertices ;

	const auto & i = geometria.indices ;

	if (v.empty() || i.empty()) { return true ; }

	const std :: uint64_t bytes_v = v.size() * sizeof(ltx :: Vertice) ;

	const std :: uint64_t bytes_i = i.size() * sizeof(std :: uint32_t) ;

	if (

		!asegurar_buffer(gpu , vertices , capacidad_vertices , bytes_v , BIND_VERTEX_BUFFER , "LTX vértices" , error) ||

		!asegurar_buffer(gpu , indices , capacidad_indices , bytes_i , BIND_INDEX_BUFFER , "LTX índices" , error)

	) { return false ; }

	gpu.ctx->UpdateBuffer(vertices , 0 , bytes_v , v.data() , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

	gpu.ctx->UpdateBuffer(indices , 0 , bytes_i , i.data() , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

	return true ;

}

std :: string hornear(const Gpu & gpu , std :: string & error) {

	const std :: string lista_ps = con_tipos(hlsl_lista_ps) ;

	const struct { const char * nombre ; SHADER_TYPE tipo ; const char * fuente ; } todos[] = {

		{ "lista_vs" , SHADER_TYPE_VERTEX , hlsl_lista_vs } ,

		{ "lista_ps" , SHADER_TYPE_PIXEL , lista_ps.c_str() } ,

		{ "marca_ps" , SHADER_TYPE_PIXEL , hlsl_marca_ps } ,

		{ "imagen_ps" , SHADER_TYPE_PIXEL , hlsl_imagen_ps } ,

		{ "pantalla_vs" , SHADER_TYPE_VERTEX , hlsl_pantalla_vs } ,

		{ "capa_ps" , SHADER_TYPE_PIXEL , hlsl_capa_ps } ,

		{ "mezcla_ps" , SHADER_TYPE_PIXEL , hlsl_mezcla_ps } ,

		{ "copia_ps" , SHADER_TYPE_PIXEL , hlsl_copia_ps } ,

	} ;

	char hexadecimal[32] ;

	std :: string datos ;

	std :: string tabla ;

	for (const auto & s : todos) {

		ShaderCreateInfo info = descripcion(s.nombre , s.tipo) ;

		info.Source = s.fuente ;

		RefCntAutoPtr<IShader> shader ;

		gpu.device->CreateShader(info , &shader) ;

		const void * bytes = nullptr ;

		Uint64 cuantos = 0 ;

		if (shader) { shader->GetBytecode(&bytes , cuantos) ; }

		if (!bytes || !cuantos) {

			error = std :: string("No se pudo compilar el shader ") + s.nombre ;

			return {} ;

		}

		const auto * palabras = static_cast<const std :: uint32_t *>(bytes) ;

		const std :: size_t n = cuantos / sizeof(std :: uint32_t) ;

		datos += std :: string("inline constexpr std :: uint32_t ") + s.nombre + "[] = {" ;

		for (std :: size_t i = 0 ; i < n ; i++) {

			std :: snprintf(hexadecimal , sizeof(hexadecimal) , "0x%x," , palabras[i]) ;

			datos += (i % 16 == 0 ? "\n" : "") ;

			datos += hexadecimal ;

		}

		datos += "\n} ;\n\n" ;

		std :: snprintf(hexadecimal , sizeof(hexadecimal) , "0x%llxull" , static_cast<unsigned long long>(huella(s.fuente))) ;

		tabla += std :: string("\tHorneado{ ") + hexadecimal + " , " + s.nombre + " , " + std :: to_string(n) + " } ,\n" ;

	}

	return

		"# pragma once\n\n"

		"// Generado con NLTX_HORNEAR (ver pintor/README.md) : los shaders ya compilados, como los dejó\n"

		"// Diligent. No se edita a mano.\n\n"

		"# include <cstddef>\n\n# include <cstdint>\n\nnamespace ltx {\n\n"

		"struct Horneado {\n\n\tstd :: uint64_t huella ;\n\n\tconst std :: uint32_t * palabras ;\n\n\tstd :: size_t cuantas ;\n\n} ;\n\n"

		+ datos +

		"inline constexpr Horneado horneados[] = {\n" + tabla + "} ;\n\n}  // namespace ltx\n" ;

}

bool Recursos :: crear(const Gpu & gpu , std :: string & error) {

	if (listos) { return true ; }

	muestras = muestras_posibles(gpu.device) ;

	formato_plantilla = elegir_plantilla(gpu.device , muestras) ;

	if (

		!crear_datos(gpu , error) ||

		!crear_pasadas_lista(gpu , error) ||

		!crear_pasadas_capa(gpu , error)

	) { return false ; }

	listos = true ;

	return true ;

}

bool Recursos :: crear_datos(const Gpu & gpu , std :: string & error) {

	const ltx :: Limites limites = ltx :: limites() ;

	BufferDesc desc ;

	desc.Name = "LTX uniformes" ;

	desc.Size = sizeof(UniformesVista) ;

	desc.Usage = USAGE_DEFAULT ;

	desc.BindFlags = BIND_UNIFORM_BUFFER ;

	desc.ImmediateContextMask = gpu.mascara() ;

	gpu.device->CreateBuffer(desc , nullptr , &uniforme_vista) ;

	desc.Size = sizeof(UniformesCapa) ;

	gpu.device->CreateBuffer(desc , nullptr , &uniforme_capa) ;

	desc.Name = "LTX tintas" ;

	desc.Size = limites.max_tintas * sizeof(ltx :: Tinta) ;

	desc.BindFlags = BIND_SHADER_RESOURCE ;

	desc.Mode = BUFFER_MODE_STRUCTURED ;

	desc.ElementByteStride = sizeof(ltx :: Tinta) ;

	gpu.device->CreateBuffer(desc , nullptr , &tintas) ;

	TextureDesc textura ;

	textura.Name = "LTX atlas" ;

	textura.Type = RESOURCE_DIM_TEX_2D ;

	textura.Width = limites.lado_atlas ;

	textura.Height = limites.lado_atlas ;

	textura.Format = TEX_FORMAT_R8_UNORM ;

	textura.Usage = USAGE_DEFAULT ;

	textura.BindFlags = BIND_SHADER_RESOURCE ;

	textura.ImmediateContextMask = gpu.mascara() ;

	gpu.device->CreateTexture(textura , nullptr , &atlas) ;

	textura.Name = "LTX rampas" ;

	textura.Width = limites.ancho_rampa ;

	textura.Height = limites.filas_rampas ;

	textura.Format = formato_color ;

	gpu.device->CreateTexture(textura , nullptr , &rampas) ;

	return (uniforme_vista && uniforme_capa && tintas && atlas && rampas) ||

		fallo(error , "No se pudieron crear los datos de LTX") ;

}

bool Recursos :: crear_pasadas_lista(const Gpu & gpu , std :: string & error) {

	const std :: string fuente = con_tipos(hlsl_lista_ps) ;

	auto vs = compilar(gpu.device , "LTX lista VS" , SHADER_TYPE_VERTEX , hlsl_lista_vs) ;

	auto ps = compilar(gpu.device , "LTX lista PS" , SHADER_TYPE_PIXEL , fuente.c_str()) ;

	auto marca = compilar(gpu.device , "LTX marca PS" , SHADER_TYPE_PIXEL , hlsl_marca_ps) ;

	auto ps_imagen = compilar(gpu.device , "LTX imagen PS" , SHADER_TYPE_PIXEL , hlsl_imagen_ps) ;

	if (!vs || !ps || !marca || !ps_imagen) { return fallo(error , "No se pudieron compilar los shaders de LTX") ; }

	const auto crear = [&](const RecetaLista & receta , Pasada & pasada) {

		GraphicsPipelineStateCreateInfo pso ;

		pso.PSODesc.Name = receta.nombre ;

		pso.PSODesc.PipelineType = PIPELINE_TYPE_GRAPHICS ;

		pso.PSODesc.ImmediateContextMask = gpu.mascara() ;

		auto & g = pso.GraphicsPipeline ;

		g.NumRenderTargets = 1 ;

		g.RTVFormats[0] = formato_color ;

		g.DSVFormat = formato_plantilla ;

		g.SmplDesc.Count = static_cast<Uint8>(muestras) ;

		g.PrimitiveTopology = PRIMITIVE_TOPOLOGY_TRIANGLE_LIST ;

		g.RasterizerDesc.CullMode = CULL_MODE_NONE ;

		g.DepthStencilDesc.DepthEnable = False ;

		g.DepthStencilDesc.DepthWriteEnable = False ;

		g.DepthStencilDesc.StencilEnable = True ;

		g.DepthStencilDesc.FrontFace = StencilOpDesc{ STENCIL_OP_KEEP , STENCIL_OP_KEEP , receta.paso , COMPARISON_FUNC_EQUAL } ;

		g.DepthStencilDesc.BackFace = g.DepthStencilDesc.FrontFace ;

		g.BlendDesc = premultiplicada() ;

		auto & rt = g.BlendDesc.RenderTargets[0] ;

		rt.RenderTargetWriteMask = receta.pinta ? COLOR_MASK_ALL : COLOR_MASK_NONE ;

		rt.BlendEnable = receta.pinta ? True : False ;

		LayoutElement layout[] = {

			{ 0 , 0 , 2 , VT_FLOAT32 , False } ,

			{ 1 , 0 , 2 , VT_FLOAT32 , False } ,

			{ 2 , 0 , 4 , VT_UINT8 , True } ,

			{ 3 , 0 , 1 , VT_UINT32 , False }

		} ;

		g.InputLayout.LayoutElements = layout ;

		g.InputLayout.NumElements = 4 ;

		SamplerDesc lineal ;

		lineal.MinFilter = lineal.MagFilter = lineal.MipFilter = FILTER_TYPE_LINEAR ;

		lineal.AddressU = lineal.AddressV = lineal.AddressW = TEXTURE_ADDRESS_CLAMP ;

		ImmutableSamplerDesc de_lista[] = {

			{ SHADER_TYPE_PIXEL , "Atlas" , lineal } ,

			{ SHADER_TYPE_PIXEL , "Rampas" , lineal }

		} ;

		ImmutableSamplerDesc de_imagen[] = { { SHADER_TYPE_PIXEL , "Imagen" , lineal } } ;

		pso.PSODesc.ResourceLayout.ImmutableSamplers = receta.textura ? de_imagen : de_lista ;

		pso.PSODesc.ResourceLayout.NumImmutableSamplers = receta.textura ? 1 : 2 ;

		ShaderResourceVariableDesc variable{ SHADER_TYPE_PIXEL , receta.textura ? receta.textura : "" , SHADER_RESOURCE_VARIABLE_TYPE_DYNAMIC } ;

		pso.PSODesc.ResourceLayout.Variables = &variable ;

		pso.PSODesc.ResourceLayout.NumVariables = receta.textura ? 1 : 0 ;

		pso.pVS = receta.vs ;

		pso.pPS = receta.ps ;

		gpu.device->CreateGraphicsPipelineState(pso , &pasada.pipeline) ;

		if (!pasada.pipeline) { return fallo(error , "No se pudo crear un pipeline de LTX") ; }

		IPipelineState * p = pasada.pipeline ;

		fijar_estatica(p , SHADER_TYPE_VERTEX , "Vista" , uniforme_vista) ;

		fijar_estatica(p , SHADER_TYPE_PIXEL , "Tintas" , tintas->GetDefaultView(BUFFER_VIEW_SHADER_RESOURCE)) ;

		fijar_estatica(p , SHADER_TYPE_PIXEL , "Atlas" , atlas->GetDefaultView(TEXTURE_VIEW_SHADER_RESOURCE)) ;

		fijar_estatica(p , SHADER_TYPE_PIXEL , "Rampas" , rampas->GetDefaultView(TEXTURE_VIEW_SHADER_RESOURCE)) ;

		p->CreateShaderResourceBinding(&pasada.enlace , true) ;

		return pasada.enlace || fallo(error , "No se pudo enlazar un pipeline de LTX") ;

	} ;

	return

		crear(RecetaLista{ "LTX dibujo" , vs , ps , true , STENCIL_OP_KEEP , nullptr } , dibujo) &&

		crear(RecetaLista{ "LTX unica" , vs , ps , true , STENCIL_OP_INCR_SAT , nullptr } , unica) &&

		crear(RecetaLista{ "LTX recorte" , vs , marca , false , STENCIL_OP_INCR_SAT , nullptr } , recorte) &&

		crear(RecetaLista{ "LTX soltar" , vs , marca , false , STENCIL_OP_DECR_SAT , nullptr } , soltar) &&

		crear(RecetaLista{ "LTX imagen" , vs , ps_imagen , true , STENCIL_OP_KEEP , "Imagen" } , imagen) ;

}

bool Recursos :: crear_pasadas_capa(const Gpu & gpu , std :: string & error) {

	auto vs = compilar(gpu.device , "LTX pantalla VS" , SHADER_TYPE_VERTEX , hlsl_pantalla_vs) ;

	auto ps_capa = compilar(gpu.device , "LTX capa PS" , SHADER_TYPE_PIXEL , hlsl_capa_ps) ;

	auto ps_mezcla = compilar(gpu.device , "LTX mezcla PS" , SHADER_TYPE_PIXEL , hlsl_mezcla_ps) ;

	auto ps_copia = compilar(gpu.device , "LTX copia PS" , SHADER_TYPE_PIXEL , hlsl_copia_ps) ;

	if (!vs || !ps_capa || !ps_mezcla || !ps_copia) { return fallo(error , "No se pudieron compilar los shaders de capa de LTX") ; }

	const auto crear = [&](const RecetaCapa & receta , Pasada & pasada) {

		GraphicsPipelineStateCreateInfo pso ;

		pso.PSODesc.Name = receta.nombre ;

		pso.PSODesc.PipelineType = PIPELINE_TYPE_GRAPHICS ;

		pso.PSODesc.ImmediateContextMask = gpu.mascara() ;

		auto & g = pso.GraphicsPipeline ;

		g.NumRenderTargets = 1 ;

		g.RTVFormats[0] = formato_color ;

		g.SmplDesc.Count = static_cast<Uint8>(receta.final_ ? 1 : muestras) ;

		g.PrimitiveTopology = PRIMITIVE_TOPOLOGY_TRIANGLE_LIST ;

		g.RasterizerDesc.CullMode = CULL_MODE_NONE ;

		g.DepthStencilDesc.DepthEnable = False ;

		g.DepthStencilDesc.DepthWriteEnable = False ;

		if (!receta.final_) {

			g.DSVFormat = formato_plantilla ;

			g.DepthStencilDesc.StencilEnable = True ;

			g.DepthStencilDesc.FrontFace = StencilOpDesc{ STENCIL_OP_KEEP , STENCIL_OP_KEEP , STENCIL_OP_KEEP , COMPARISON_FUNC_EQUAL } ;

			g.DepthStencilDesc.BackFace = g.DepthStencilDesc.FrontFace ;

		}

		g.BlendDesc = premultiplicada() ;

		g.BlendDesc.RenderTargets[0].BlendEnable = receta.mezclada ? True : False ;

		ShaderResourceVariableDesc variables[] = {

			{ SHADER_TYPE_PIXEL , "Capa" , SHADER_RESOURCE_VARIABLE_TYPE_DYNAMIC } ,

			{ SHADER_TYPE_PIXEL , receta.segunda ? receta.segunda : "" , SHADER_RESOURCE_VARIABLE_TYPE_DYNAMIC }

		} ;

		pso.PSODesc.ResourceLayout.Variables = variables ;

		pso.PSODesc.ResourceLayout.NumVariables = receta.segunda ? 2 : 1 ;

		pso.pVS = receta.vs ;

		pso.pPS = receta.ps ;

		gpu.device->CreateGraphicsPipelineState(pso , &pasada.pipeline) ;

		if (!pasada.pipeline) { return fallo(error , "No se pudo crear un pipeline de capa de LTX") ; }

		fijar_estatica(pasada.pipeline , SHADER_TYPE_PIXEL , "Composicion" , uniforme_capa) ;

		pasada.pipeline->CreateShaderResourceBinding(&pasada.enlace , true) ;

		return pasada.enlace || fallo(error , "No se pudo enlazar un pipeline de capa de LTX") ;

	} ;

	return

		crear(RecetaCapa{ "LTX capa" , vs , ps_capa , "Mascara" , true , false } , capa) &&

		crear(RecetaCapa{ "LTX mezcla" , vs , ps_mezcla , "Fondo" , false , false } , mezcla) &&

		crear(RecetaCapa{ "LTX copia" , vs , ps_copia , nullptr , false , true } , copia) ;

}

void Recursos :: acomodar(std :: uint32_t w , std :: uint32_t h) {

	if (w <= ancho && h <= alto) { return ; }

	const auto salto = [](std :: uint32_t v) { return (v + 255u) & ~255u ; } ;

	objetivos.clear() ;

	ancho = std :: max(ancho , salto(w)) ;

	alto = std :: max(alto , salto(h)) ;

}

bool Recursos :: subir_imagenes(const Gpu & gpu , const ltx :: Marco & marco , std :: string & error) {

	std :: unordered_set<std :: uint64_t> pintadas ;

	for (std :: size_t i = 0 ; i < marco.imagenes() ; i++) {

		const std :: uint64_t id = marco.id_imagen(i) ;

		pintadas.insert(id) ;

		if (imagenes.count(id) != 0) { continue ; }

		TextureDesc desc ;

		desc.Name = "LTX imagen" ;

		desc.Type = RESOURCE_DIM_TEX_2D ;

		desc.Width = marco.ancho_imagen(i) ;

		desc.Height = marco.alto_imagen(i) ;

		desc.Format = formato_color ;

		// todos sus mipmaps : una foto reducida a la columna no debe salir a dientes

		desc.MipLevels = 0 ;

		desc.BindFlags = BIND_SHADER_RESOURCE | BIND_RENDER_TARGET ;

		desc.MiscFlags = MISC_TEXTURE_FLAG_GENERATE_MIPS ;

		desc.ImmediateContextMask = gpu.mascara() ;

		RefCntAutoPtr<ITexture> textura ;

		gpu.device->CreateTexture(desc , nullptr , &textura) ;

		if (!textura) { return fallo(error , "No se pudo crear la textura de una imagen de LTX") ; }

		const auto rgba = marco.rgba_imagen(i) ;

		TextureSubResData sub ;

		sub.pData = rgba.data() ;

		sub.Stride = desc.Width * 4 ;

		const Box caja(0 , desc.Width , 0 , desc.Height , 0 , 1) ;

		gpu.ctx->UpdateTexture(textura , 0 , 0 , caja , sub , RESOURCE_STATE_TRANSITION_MODE_NONE , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		gpu.ctx->GenerateMips(textura->GetDefaultView(TEXTURE_VIEW_SHADER_RESOURCE)) ;

		imagenes.emplace(id , std :: move(textura)) ;

	}

	for (auto i = imagenes.begin() ; i != imagenes.end() ; ) {

		i = pintadas.count(i->first) != 0 ? std :: next(i) : imagenes.erase(i) ;

	}

	return true ;

}

Objetivo * Recursos :: objetivo(const Gpu & gpu , std :: size_t i , std :: string & error) {

	while (objetivos.size() <= i) {

		auto nuevo = std :: make_unique<Objetivo>() ;

		if (!nuevo->crear(gpu , FormatoObjetivo{ ancho , alto , muestras , formato_plantilla } , error)) { return nullptr ; }

		objetivos.push_back(std :: move(nuevo)) ;

	}

	return objetivos[i].get() ;

}

}  // namespace ltx
