// La familia de N-LTX : su canal, su fábrica y el método «medidas» (el resto lo
// atiende el registro de texturas, N-CNX/gpu/linux/texturas.cc).

# include "familia_ltx.h"

# include <cstring>

# include "gpu_ltx.h"

namespace {

bool metodo(LlamadaTextura & llamada) {

	if (std :: strcmp(llamada.nombre , "medidas") != 0) return false ;

	const MedidasLtx m = llamada.gpu ? static_cast<GpuLtx *>(llamada.gpu)->medidas() : MedidasLtx{} ;

	g_autoptr(FlValue) value = fl_value_new_list() ;

	fl_value_append_take(value , fl_value_new_int(m.elementos)) ;

	fl_value_append_take(value , fl_value_new_float(m.fotogramas_por_segundo)) ;

	fl_value_append_take(value , fl_value_new_float(m.ms_carril)) ;

	fl_method_call_respond_success(llamada.call , value , nullptr) ;

	return true ;

}

}  // namespace

FamiliaTexturas familia_ltx() {

	return FamiliaTexturas{

		"ltx/textura" ,

		1 ,

		[](Carril & carril) -> std :: unique_ptr<GpuRenderer> { return std :: make_unique<GpuLtx>(carril) ; } ,

		metodo ,

		nullptr

	} ;

}
