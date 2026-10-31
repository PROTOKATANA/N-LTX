// La familia de la prueba masiva : su canal y su fábrica (el resto lo atiende el
// registro de texturas, N-CNX/gpu/linux/texturas.cc).

# include "familia_masiva.h"

# include "gpu_masiva.h"

FamiliaTexturas familia_masiva() {

	return FamiliaTexturas{

		"ltx/masiva" ,

		1 ,

		[](Carril & carril) -> std :: unique_ptr<GpuRenderer> { return std :: make_unique<GpuMasiva>(carril) ; } ,

		[](LlamadaTextura &) { return false ; } ,

		nullptr

	} ;

}
