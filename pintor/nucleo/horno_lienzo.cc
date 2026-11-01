# include "horno_lienzo.h"

# include <cstdint>

# include <cstdio>

# include <vector>

# include "pintor_lienzo.h"

# include "sombreadores_horneados.h"

# include "sombreadores_lienzo.h"

using namespace Diligent ;

namespace ltx {

namespace {

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

// Un shader que hornear, y el nombre que lleva en sombreadores_horneados.h

struct Sombreador {

	const char * nombre ;

	SHADER_TYPE tipo ;

	const char * fuente ;

} ;

// Su SPIR-V, compilado del HLSL aunque haya algo horneado. Vacío si no se pudo

std :: vector<std :: uint32_t> spirv(const Gpu & gpu , const Sombreador & s) {

	ShaderCreateInfo info = descripcion(s.nombre , s.tipo) ;

	info.Source = s.fuente ;

	RefCntAutoPtr<IShader> shader ;

	gpu.device->CreateShader(info , &shader) ;

	const void * bytes = nullptr ;

	Uint64 cuantos = 0 ;

	if (shader) { shader->GetBytecode(&bytes , cuantos) ; }

	const auto * palabras = static_cast<const std :: uint32_t *>(bytes) ;

	return bytes ? std :: vector<std :: uint32_t>(palabras , palabras + cuantos / sizeof(std :: uint32_t)) : std :: vector<std :: uint32_t>{} ;

}

std :: string formateado(const char * formato , unsigned long long valor) {

	char texto[32] ;

	std :: snprintf(texto , sizeof(texto) , formato , valor) ;

	return texto ;

}

std :: string lista_de_palabras(const Sombreador & s , const std :: vector<std :: uint32_t> & palabras) {

	std :: string texto = std :: string("inline constexpr std :: uint32_t ") + s.nombre + "[] = {" ;

	for (std :: size_t i = 0 ; i < palabras.size() ; i++) {

		texto += (i % 16 == 0 ? "\n" : "") ;

		texto += formateado("0x%llx," , palabras[i]) ;

	}

	return texto + "\n} ;\n\n" ;

}

std :: string entrada_de_la_tabla(const Sombreador & s , std :: size_t palabras) {

	return std :: string("\tHorneado{ ") + formateado("0x%llxull" , huella(s.fuente)) + " , " + s.nombre + " , " + std :: to_string(palabras) + " } ,\n" ;

}

std :: string archivo(const std :: string & datos , const std :: string & tabla) {

	return

		"# pragma once\n\n"

		"// Generado con NLTX_HORNEAR (ver pintor/README.md) : los shaders ya compilados, como los dejó\n"

		"// Diligent. No se edita a mano.\n\n"

		"# include <cstddef>\n\n# include <cstdint>\n\nnamespace ltx {\n\n"

		"struct Horneado {\n\n\tstd :: uint64_t huella ;\n\n\tconst std :: uint32_t * palabras ;\n\n\tstd :: size_t cuantas ;\n\n} ;\n\n"

		+ datos +

		"inline constexpr Horneado horneados[] = {\n" + tabla + "} ;\n\n}  // namespace ltx\n" ;

}

}  // namespace

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

std :: string hornear(const Gpu & gpu , std :: string & error) {

	const std :: string lista_ps = con_tipos(hlsl_lista_ps) ;

	const Sombreador todos[] = {

		{ "lista_vs" , SHADER_TYPE_VERTEX , hlsl_lista_vs } ,

		{ "lista_ps" , SHADER_TYPE_PIXEL , lista_ps.c_str() } ,

		{ "marca_ps" , SHADER_TYPE_PIXEL , hlsl_marca_ps } ,

		{ "imagen_ps" , SHADER_TYPE_PIXEL , hlsl_imagen_ps } ,

		{ "pantalla_vs" , SHADER_TYPE_VERTEX , hlsl_pantalla_vs } ,

		{ "capa_ps" , SHADER_TYPE_PIXEL , hlsl_capa_ps } ,

		{ "mezcla_ps" , SHADER_TYPE_PIXEL , hlsl_mezcla_ps } ,

		{ "copia_ps" , SHADER_TYPE_PIXEL , hlsl_copia_ps } ,

	} ;

	std :: string datos ;

	std :: string tabla ;

	for (const Sombreador & s : todos) {

		const std :: vector<std :: uint32_t> palabras = spirv(gpu , s) ;

		if (palabras.empty()) {

			error = std :: string("No se pudo compilar el shader ") + s.nombre ;

			return {} ;

		}

		datos += lista_de_palabras(s , palabras) ;

		tabla += entrada_de_la_tabla(s , palabras.size()) ;

	}

	return archivo(datos , tabla) ;

}

}  // namespace ltx
