# pragma once

# include <cstdint>

# include <memory>

# include <string>

# include <unordered_map>

# include <vector>

# include "DeviceContext.h"

# include "RefCntAutoPtr.hpp"

# include "RenderDevice.h"

# include "pintor/src/puente_lienzo.rs.h"

namespace ltx {

inline constexpr Diligent :: TEXTURE_FORMAT formato_color = Diligent :: TEX_FORMAT_RGBA8_UNORM ;

struct UniformesVista {

	float ancho ;

	float alto ;

	float x ;

	float y ;

} ;

struct UniformesCapa {

	float opacidad ;

	std :: uint32_t mezcla ;

	std :: uint32_t con_mascara ;

	std :: uint32_t reservado ;

} ;

// El dispositivo y el contexto con que se pinta : los pone quien integra N-LTX

struct Gpu {

	Diligent :: IRenderDevice * device ;

	Diligent :: IDeviceContext * ctx ;

	// Cada recurso es solo de ese contexto : Diligent libera un recurso cuando
	// avanzan todos los contextos de su máscara, y con más bits lo soltado
	// esperaría a un contexto quieto

	Diligent :: Uint64 mascara() const { return Diligent :: Uint64{ 1 } << ctx->GetDesc().ContextId ; }

} ;

// Un pipeline y su enlace de recursos

struct Pasada {

	Diligent :: RefCntAutoPtr<Diligent :: IPipelineState> pipeline ;

	Diligent :: RefCntAutoPtr<Diligent :: IShaderResourceBinding> enlace ;

} ;

struct FormatoObjetivo {

	std :: uint32_t ancho ;

	std :: uint32_t alto ;

	std :: uint32_t muestras ;

	Diligent :: TEXTURE_FORMAT plantilla ;

} ;

// Una textura de color con MSAA, su stencil y dónde se resuelve : una por cada
// capa abierta a la vez

struct Objetivo {

	Diligent :: RefCntAutoPtr<Diligent :: ITexture> color ;

	Diligent :: RefCntAutoPtr<Diligent :: ITexture> plantilla ;

	Diligent :: RefCntAutoPtr<Diligent :: ITexture> resuelta ;

	bool crear(const Gpu & gpu , const FormatoObjetivo & formato , std :: string & error) ;

} ;

struct Geometria {

	rust :: Slice<const ltx :: Vertice> vertices ;

	rust :: Slice<const std :: uint32_t> indices ;

} ;

// Los búferes de una lista

struct Malla {

	Diligent :: RefCntAutoPtr<Diligent :: IBuffer> vertices ;

	Diligent :: RefCntAutoPtr<Diligent :: IBuffer> indices ;

	std :: uint64_t capacidad_vertices = 0 ;

	std :: uint64_t capacidad_indices = 0 ;

	bool subir(const Gpu & gpu , const Geometria & geometria , std :: string & error) ;

} ;

// Todo lo que vive en la GPU : uno por dispositivo

struct Recursos {

	Diligent :: RefCntAutoPtr<Diligent :: IBuffer> uniforme_vista ;

	Diligent :: RefCntAutoPtr<Diligent :: IBuffer> uniforme_capa ;

	Diligent :: RefCntAutoPtr<Diligent :: IBuffer> tintas ;

	Diligent :: RefCntAutoPtr<Diligent :: ITexture> atlas ;

	Diligent :: RefCntAutoPtr<Diligent :: ITexture> rampas ;

	Pasada dibujo ;

	Pasada unica ;

	Pasada recorte ;

	Pasada soltar ;

	Pasada imagen ;

	Pasada capa ;

	Pasada mezcla ;

	Pasada copia ;

	std :: vector<std :: unique_ptr<Objetivo>> objetivos ;

	std :: uint32_t ancho = 0 ;

	std :: uint32_t alto = 0 ;

	std :: uint32_t muestras = 0 ;

	Diligent :: TEXTURE_FORMAT formato_plantilla = Diligent :: TEX_FORMAT_UNKNOWN ;

	Malla base ;

	Malla vivos ;

	// Las imágenes subidas, por su id : republicar no las vuelve a subir

	std :: unordered_map<std :: uint64_t , Diligent :: RefCntAutoPtr<Diligent :: ITexture>> imagenes ;

	std :: uint64_t generacion_base = 0 ;

	std :: uint64_t generacion_vivos = 0 ;

	bool listos = false ;

	bool crear(const Gpu & gpu , std :: string & error) ;

	// Solo crecen, y a saltos : encoger o estirar la ventana no crea texturas
	// en cada paso, y una ventana pequeña no paga el monitor entero

	void acomodar(std :: uint32_t w , std :: uint32_t h) ;

	Objetivo * objetivo(const Gpu & gpu , std :: size_t i , std :: string & error) ;

	// Sube lo que falte de lo que pinta `marco` y suelta lo que ya no pinta

	bool subir_imagenes(const Gpu & gpu , const ltx :: Marco & marco , std :: string & error) ;

private:

	bool crear_datos(const Gpu & gpu , std :: string & error) ;

	bool crear_pasadas_lista(const Gpu & gpu , std :: string & error) ;

	bool crear_pasadas_capa(const Gpu & gpu , std :: string & error) ;

} ;

}  // namespace ltx
