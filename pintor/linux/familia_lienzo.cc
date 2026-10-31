// La familia del lienzo : su canal, su fábrica y sus métodos propios (el resto
// lo atiende el registro de texturas, N-CNX/gpu/linux/texturas.cc).

# include "familia_lienzo.h"

# include <cstring>

# include "gpu_lienzo.h"

# include "lienzo.h"

namespace {

// El tamaño de las superficies : el monitor más grande

Lienzo lienzo ;

void responder(LlamadaTextura & llamada , FlValue * value) { fl_method_call_respond_success(llamada.call , value , nullptr) ; }

// [ancho , alto , escala , y] : los dos primeros, enteros

bool fijar_vista(LlamadaTextura & llamada) {

	FlValue * a = llamada.args ;

	if (!a || fl_value_get_type(a) != FL_VALUE_TYPE_LIST || fl_value_get_length(a) != 4) { return false ; }

	if (llamada.gpu) {

		static_cast<GpuLienzo *>(llamada.gpu)->fijar_vista(

			static_cast<std :: uint32_t>(fl_value_get_int(fl_value_get_list_value(a , 0))) ,

			static_cast<std :: uint32_t>(fl_value_get_int(fl_value_get_list_value(a , 1))) ,

			fl_value_get_float(fl_value_get_list_value(a , 2)) ,

			fl_value_get_float(fl_value_get_list_value(a , 3))

		) ;

		llamada.marcar() ;

	}

	responder(llamada , nullptr) ;

	return true ;

}

bool metodo(LlamadaTextura & llamada) {

	if (std :: strcmp(llamada.nombre , "vista") == 0) { return fijar_vista(llamada) ; }

	if (std :: strcmp(llamada.nombre , "lienzo") == 0) {

		g_autoptr(FlValue) value = fl_value_new_list() ;

		fl_value_append_take(value , fl_value_new_int(lienzo.ancho)) ;

		fl_value_append_take(value , fl_value_new_int(lienzo.alto)) ;

		responder(llamada , value) ;

		return true ;

	}

	if (std :: strcmp(llamada.nombre , "volcar") == 0) {

		if (llamada.gpu && fl_value_get_type(llamada.args) == FL_VALUE_TYPE_STRING) {

			static_cast<GpuLienzo *>(llamada.gpu)->volcar(fl_value_get_string(llamada.args)) ;

			llamada.marcar() ;

		}

		responder(llamada , nullptr) ;

		return true ;

	}

	if (std :: strcmp(llamada.nombre , "medidas") != 0) { return false ; }

	const MedidasLienzo m = llamada.gpu ? static_cast<GpuLienzo *>(llamada.gpu)->medidas() : MedidasLienzo{} ;

	g_autoptr(FlValue) value = fl_value_new_list() ;

	fl_value_append_take(value , fl_value_new_int(m.vertices)) ;

	fl_value_append_take(value , fl_value_new_float(m.ms_carril)) ;

	responder(llamada , value) ;

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
