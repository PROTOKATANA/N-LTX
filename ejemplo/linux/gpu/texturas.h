# pragma once

# include <flutter_linux/flutter_linux.h>

# include <cstddef>

# include <functional>

# include <memory>

# include <vector>

# include "carril.h"

# include "gpu_renderer.h"

// Lo que recibe un método propio de una familia : la llamada, sus argumentos,
// el renderizador de la textura que nombra (nulo si aún no está creada) y
// cómo pedir un fotograma (solicitar y marcar la textura)

struct LlamadaTextura {

	const char * nombre ;

	FlValue * args ;

	GpuRenderer * gpu ;

	FlMethodCall * call ;

	std :: function<void()> marcar ;

} ;

// Una familia de texturas GPU con su canal de método
// y su carril (su hilo de render, ver carril.h). El registro atiende
// create(id) → textureId, mark(id) y dispose(id) (id : el argumento si es un
// entero, si no 0); los demás métodos van a `metodo`, que responde la llamada
// y devuelve true, o false si no es suyo (no implementado)

struct FamiliaTexturas {

	const char * canal ;

	// Superficies a la vez, como mucho

	std :: size_t maximo ;

	// Un renderizador nuevo, que pinta en el carril de la familia, o nullptr
	// si no se puede (sin libhub.so…)

	std :: function<std :: unique_ptr<GpuRenderer>(Carril &)> fabrica ;

	std :: function<bool(LlamadaTextura &)> metodo ;

	// Al apagar, cuando ya no tiene texturas (puede ser nulo)

	std :: function<void()> cerrar ;

	// Crear un renderizador al registrar la familia y precalentarlo (ver
	// GpuRenderer :: precalentar) : el primer create lo encuentra listo

	bool precalentar = false ;

} ;

// Registra las familias de la app, cada una con su canal y su carril. Al
// fallar el render de una textura, llama a Dart : rendererError {id , message}
void registrar_texturas(FlView * view , std :: vector<FamiliaTexturas> familias) ;

void cerrar_texturas() ;
