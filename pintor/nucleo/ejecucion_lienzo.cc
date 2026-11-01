# include "ejecucion_lienzo.h"

using namespace Diligent ;

namespace ltx {

namespace {

constexpr float transparente[4] = { 0.0f , 0.0f , 0.0f , 0.0f } ;

ITextureView * vista_srv(ITexture * t) { return t->GetDefaultView(TEXTURE_VIEW_SHADER_RESOURCE) ; }

}  // namespace

Ejecucion :: Ejecucion(const Gpu & gpu , Recursos & recursos , const Recuadro & recuadro) :

	gpu_(gpu) , r_(recursos) , recuadro_(recuadro) {}

bool Ejecucion :: abrir_base(const float * fondo , std :: string & error) {

	if (!r_.objetivo(gpu_ , 0 , error)) { return false ; }

	fijar(0 , fondo) ;

	return true ;

}

bool Ejecucion :: lista(rust :: Slice<const ltx :: Comando> comandos , Malla & malla , std :: string & error) {

	if (comandos.empty() || !malla.vertices) { return true ; }

	IDeviceContext * ctx = gpu_.ctx ;

	const Uint64 offset = 0 ;

	IBuffer * buffers[] = { malla.vertices } ;

	ctx->SetVertexBuffers(0 , 1 , buffers , &offset , RESOURCE_STATE_TRANSITION_MODE_TRANSITION , SET_VERTEX_BUFFERS_FLAG_RESET) ;

	ctx->SetIndexBuffer(malla.indices , 0 , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

	for (std :: size_t i = 0 ; i < comandos.size() ; i++) {

		const ltx :: Comando & c = comandos[i] ;

		switch (c.orden) {

			case ltx :: Orden :: Dibujar : indices(r_.dibujo , c) ; break ;

			case ltx :: Orden :: Unica : indices(r_.unica , c) ; break ;

			case ltx :: Orden :: Recortar : indices(r_.recorte , c) ; break ;

			case ltx :: Orden :: Soltar : indices(r_.soltar , c) ; break ;

			case ltx :: Orden :: Imagen : imagen(c) ; break ;

			case ltx :: Orden :: AbrirCapa :

				// lo que no se ve no gasta una textura

				if (c.y1 < recuadro_.y0 || c.y0 > recuadro_.y1) {

					i = c.salto ;

					break ;

				}

				if (!abrir_capa(error)) { return false ; }

				break ;

			case ltx :: Orden :: CerrarMascara : cerrar_mascara() ; break ;

			case ltx :: Orden :: CerrarCapa : cerrar_capa(c) ; break ;

			default : break ;

		}

	}

	return true ;

}

ITexture * Ejecucion :: resolver_base() {

	resolver(0) ;

	return r_.objetivos[0]->resuelta ;

}

void Ejecucion :: fijar(std :: size_t i , const float * limpiar) {

	IDeviceContext * ctx = gpu_.ctx ;

	Objetivo & o = * r_.objetivos[i] ;

	ITextureView * rtv = o.color->GetDefaultView(TEXTURE_VIEW_RENDER_TARGET) ;

	ITextureView * dsv = o.plantilla->GetDefaultView(TEXTURE_VIEW_DEPTH_STENCIL) ;

	ctx->SetRenderTargets(1 , &rtv , dsv , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

	if (limpiar) {

		ctx->ClearRenderTarget(rtv , limpiar , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		ctx->ClearDepthStencil(dsv , CLEAR_STENCIL_FLAG , 1.0f , 0 , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

	}

	Viewport vista ;

	vista.Width = static_cast<float>(recuadro_.ancho) ;

	vista.Height = static_cast<float>(recuadro_.alto) ;

	ctx->SetViewports(1 , &vista , r_.ancho , r_.alto) ;

	actual_ = nullptr ;

}

void Ejecucion :: usar(const Pasada & pasada) {

	if (pasada.pipeline == actual_) { return ; }

	gpu_.ctx->SetPipelineState(pasada.pipeline) ;

	gpu_.ctx->CommitShaderResources(pasada.enlace , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

	actual_ = pasada.pipeline ;

}

void Ejecucion :: indices(const Pasada & pasada , const ltx :: Comando & c) {

	if (c.cuenta == 0) { return ; }

	usar(pasada) ;

	gpu_.ctx->SetStencilRef(c.nivel) ;

	DrawIndexedAttribs draw ;

	draw.NumIndices = c.cuenta ;

	draw.IndexType = VT_UINT32 ;

	draw.FirstIndexLocation = c.inicio ;

	gpu_.ctx->DrawIndexed(draw) ;

}

void Ejecucion :: imagen(const ltx :: Comando & c) {

	const auto subida = r_.imagenes.find(c.imagen) ;

	if (subida == r_.imagenes.end()) { return ; }

	r_.imagen.enlace->GetVariableByName(SHADER_TYPE_PIXEL , "Imagen")->Set(vista_srv(subida->second)) ;

	// el enlace ha cambiado aunque la pasada sea la misma

	actual_ = nullptr ;

	indices(r_.imagen , c) ;

	actual_ = nullptr ;

}

bool Ejecucion :: abrir_capa(std :: string & error) {

	if (!r_.objetivo(gpu_ , profundidad_ + 1 , error)) { return false ; }

	profundidad_++ ;

	fijar(profundidad_ , transparente) ;

	return true ;

}

void Ejecucion :: resolver(std :: size_t i) {

	Objetivo & o = * r_.objetivos[i] ;

	gpu_.ctx->SetRenderTargets(0 , nullptr , nullptr , RESOURCE_STATE_TRANSITION_MODE_NONE) ;

	ResolveTextureSubresourceAttribs resolucion ;

	resolucion.SrcTextureTransitionMode = RESOURCE_STATE_TRANSITION_MODE_TRANSITION ;

	resolucion.DstTextureTransitionMode = RESOURCE_STATE_TRANSITION_MODE_TRANSITION ;

	gpu_.ctx->ResolveTextureSubresource(o.color , o.resuelta , resolucion) ;

}

void Ejecucion :: cerrar_mascara() {

	resolver(profundidad_) ;

	mascara_ = r_.objetivos[profundidad_]->resuelta ;

	profundidad_-- ;

}

void Ejecucion :: cerrar_capa(const ltx :: Comando & c) {

	IDeviceContext * ctx = gpu_.ctx ;

	resolver(profundidad_) ;

	ITexture * capa = r_.objetivos[profundidad_]->resuelta ;

	profundidad_-- ;

	const bool mezclada = c.mezcla != 0 ;

	if (mezclada) { resolver(profundidad_) ; }

	const UniformesCapa u{ c.opacidad , c.mezcla , c.con_mascara ? 1u : 0u , 0u } ;

	ctx->UpdateBuffer(r_.uniforme_capa , 0 , sizeof(u) , &u , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

	ITexture * segunda = mezclada ? r_.objetivos[profundidad_]->resuelta.RawPtr() : (mascara_ && c.con_mascara ? mascara_ : capa) ;

	const Pasada & pasada = mezclada ? r_.mezcla : r_.capa ;

	IShaderResourceBinding * enlace = pasada.enlace ;

	enlace->GetVariableByName(SHADER_TYPE_PIXEL , "Capa")->Set(vista_srv(capa)) ;

	enlace->GetVariableByName(SHADER_TYPE_PIXEL , mezclada ? "Fondo" : "Mascara")->Set(vista_srv(segunda)) ;

	ctx->SetPipelineState(pasada.pipeline) ;

	ctx->CommitShaderResources(enlace , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

	fijar(profundidad_ , nullptr) ;

	ctx->SetPipelineState(pasada.pipeline) ;

	ctx->SetStencilRef(c.nivel) ;

	DrawAttribs draw ;

	draw.NumVertices = 3 ;

	ctx->Draw(draw) ;

	mascara_ = nullptr ;

	actual_ = nullptr ;

}

}  // namespace ltx
