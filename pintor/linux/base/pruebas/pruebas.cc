// Pruebas de las piezas puras de la base GPU (sin Flutter, GLib ni
// Diligent) : el relevo de las imágenes entre el carril y el raster. Sin
// biblioteca de pruebas : cada comprobación que falla lo dice y el programa
// sale con error. Se compilan y corren con pruebas/probar.sh

# include <cstdio>

# include "relevo.h"

namespace {

int fallos = 0 ;

void comprobar(bool bien , const char * que) {

	if (!bien) {

		std :: printf("FALLA : %s\n" , que) ;

		fallos++ ;

	}

}

void sin_nada_listo_no_cambia_nada() {

	Relevo r(2) ;

	const Relevo :: Cambio c = r.tomar_lista() ;

	comprobar(c.mostrar == -1 && c.soltar == -1 , "sin lista, el raster no cambia nada") ;

	comprobar(r.mostrada() == -1 , "al principio no hay nada a la vista") ;

}

void la_primera_se_muestra_sin_soltar_nada() {

	Relevo r(2) ;

	const int a = r.tomar_libre() ;

	comprobar(a >= 0 , "hay una libre") ;

	comprobar(!r.consumir_espera(a) , "una imagen nueva no espera a GL") ;

	r.publicar(a) ;

	const Relevo :: Cambio c = r.tomar_lista() ;

	comprobar(c.mostrar == a && c.soltar == -1 , "se muestra la primera y no se suelta nada") ;

	comprobar(r.mostrada() == a , "la primera queda a la vista") ;

}

void la_que_se_suelta_espera_a_gl_una_vez() {

	Relevo r(2) ;

	const int a = r.tomar_libre() ;

	r.publicar(a) ;

	r.tomar_lista() ;

	// a la vista a; se pinta la otra

	const int b = r.tomar_libre() ;

	comprobar(b >= 0 && b != a , "se pinta la que no está a la vista") ;

	comprobar(!r.hay_lista() , "pintándose, aún no hay lista") ;

	r.publicar(b) ;

	comprobar(r.hay_lista() && r.mostrada() == a , "publicada, hay lista y la vieja sigue a la vista") ;

	const Relevo :: Cambio c = r.tomar_lista() ;

	comprobar(c.mostrar == b && c.soltar == a , "se muestra la nueva y se suelta la vieja") ;

	// la vieja, otra vez libre, con su espera

	const int otra = r.tomar_libre() ;

	comprobar(otra == a , "la soltada vuelve a pintarse") ;

	comprobar(r.consumir_espera(otra) , "y antes espera el semáforo de GL") ;

	comprobar(!r.consumir_espera(otra) , "la espera se consume una sola vez") ;

}

void con_una_lista_esperando_no_se_pinta_otra() {

	Relevo r(2) ;

	const int a = r.tomar_libre() ;

	comprobar(r.tomar_libre() == -1 , "mientras se pinta una, no se empieza otra") ;

	r.publicar(a) ;

	comprobar(r.tomar_libre() == -1 , "con una lista sin coger, tampoco : el ritmo lo marca el raster") ;

	r.tomar_lista() ;

	comprobar(r.tomar_libre() >= 0 , "cogida la lista, se puede pintar la siguiente") ;

}

void devolver_no_pierde_la_espera() {

	Relevo r(2) ;

	const int a = r.tomar_libre() ;

	r.publicar(a) ;

	r.tomar_lista() ;

	const int b = r.tomar_libre() ;

	r.publicar(b) ;

	r.tomar_lista() ;

	// a, soltada con espera; se toma y no se pinta nada

	const int c = r.tomar_libre() ;

	comprobar(c == a , "se toma la soltada") ;

	r.devolver(c) ;

	comprobar(r.mostrada() == b , "devolver no toca la que está a la vista") ;

	const int d = r.tomar_libre() ;

	comprobar(d == a && r.consumir_espera(d) , "devuelta sin pintar, conserva su espera") ;

}

void reiniciar_lo_deja_todo_libre() {

	Relevo r(2) ;

	const int a = r.tomar_libre() ;

	r.publicar(a) ;

	r.tomar_lista() ;

	r.reiniciar() ;

	comprobar(r.mostrada() == -1 , "imágenes nuevas : nada a la vista") ;

	const int b = r.tomar_libre() ;

	comprobar(b >= 0 && !r.consumir_espera(b) , "y libres sin espera") ;

}

// Un índice fuera del relevo (-1 : ninguna tomada) no toca nada
void un_indice_fuera_no_toca_nada() {

	Relevo r(2) ;

	r.publicar(-1) ;

	r.publicar(2) ;

	r.devolver(-1) ;

	r.devolver(7) ;

	comprobar(!r.consumir_espera(-1) && !r.consumir_espera(2) , "fuera del relevo : sin espera") ;

	comprobar(!r.hay_lista() && r.mostrada() == -1 , "fuera del relevo : nada cambió") ;

	comprobar(r.tomar_libre() >= 0 , "fuera del relevo : sigue libre") ;

}

}  // namespace

int main() {

	sin_nada_listo_no_cambia_nada() ;

	la_primera_se_muestra_sin_soltar_nada() ;

	la_que_se_suelta_espera_a_gl_una_vez() ;

	con_una_lista_esperando_no_se_pinta_otra() ;

	devolver_no_pierde_la_espera() ;

	reiniciar_lo_deja_todo_libre() ;

	un_indice_fuera_no_toca_nada() ;

	if (fallos) {

		std :: printf("%d comprobaciones fallaron\n" , fallos) ;

		return 1 ;

	}

	std :: printf("pruebas de la base GPU : todo bien\n") ;

	return 0 ;

}
