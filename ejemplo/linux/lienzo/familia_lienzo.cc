// La familia del lienzo : su canal, su fábrica y el método «lienzo» (el resto lo
// atiende el registro de texturas, gpu/texturas.cc).

# include "familia_lienzo.h"

# include <cstring>

# include "gpu_lienzo.h"

# include "lienzo.h"

namespace {

// El tamaño de las superficies : el monitor más grande

Lienzo lienzo ;

// Lo único que Flutter no puede saber por sí mismo : el tamaño de la textura.
// El resto del lienzo viaja por rinf hasta Rust, nunca por este hilo

bool metodo(LlamadaTextura & llamada) {

	if (std :: strcmp(llamada.nombre , "lienzo") != 0) { return false ; }

	g_autoptr(FlValue) value = fl_value_new_list() ;

	fl_value_append_take(value , fl_value_new_int(lienzo.ancho)) ;

	fl_value_append_take(value , fl_value_new_int(lienzo.alto)) ;

	fl_method_call_respond_success(llamada.call , value , nullptr) ;

	return true ;

}

}  // namespace

FamiliaTexturas familia_lienzo() {

	lienzo = lienzo_de_monitores() ;

	return FamiliaTexturas{

		"ltx/lienzo" ,

		1 ,

		[](Carril & carril) -> std :: unique_ptr<GpuRenderer> {

			return std :: make_unique<GpuLienzo>(carril , lienzo.ancho , lienzo.alto) ;

		} ,

		metodo ,

		nullptr

	} ;

}
