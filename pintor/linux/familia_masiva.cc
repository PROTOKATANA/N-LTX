// La familia de la prueba masiva : su canal, su fábrica y el método «medidas» (el
// resto lo atiende el registro de texturas, N-CNX/gpu/linux/texturas.cc).

# include "familia_masiva.h"

# include <cstring>

# include "gpu_masiva.h"

namespace {

bool metodo(LlamadaTextura & llamada) {

	if (std :: strcmp(llamada.nombre , "medidas") != 0) { return false ; }

	const MedidasMasiva m = llamada.gpu ? static_cast<GpuMasiva *>(llamada.gpu)->medidas() : MedidasMasiva{} ;

	g_autoptr(FlValue) value = fl_value_new_list() ;

	fl_value_append_take(value , fl_value_new_int(m.elementos)) ;

	fl_value_append_take(value , fl_value_new_float(m.fotogramas_por_segundo)) ;

	fl_value_append_take(value , fl_value_new_float(m.ms_carril)) ;

	fl_method_call_respond_success(llamada.call , value , nullptr) ;

	return true ;

}

}  // namespace

FamiliaTexturas familia_masiva() {

	return FamiliaTexturas{

		"ltx/masiva" ,

		1 ,

		[](Carril & carril) -> std :: unique_ptr<GpuRenderer> { return std :: make_unique<GpuMasiva>(carril) ; } ,

		metodo ,

		nullptr

	} ;

}
