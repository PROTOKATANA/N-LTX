# pragma once

# include <cstddef>

# include <mutex>

# include <vector>

// El relevo de las imágenes de una superficie entre quien las pinta (el hilo
// de su carril) y quien las muestra (el hilo raster de Flutter). Pieza pura :
// solo lleva qué imagen está en cada estado; los semáforos y las texturas los
// pone gpu_surface. Se puede llamar desde los dos hilos.
//
//   libre → pintando → lista → mostrada → libre (con espera)
//
// Una imagen que dejó de mostrarse vuelve libre «con espera» : GL la soltó
// con su semáforo, y quien la pinte otra vez tiene que esperarlo antes de
// escribir. La que se pinta sin publicar (no había nada nuevo) vuelve libre
// tal como estaba. Nunca hay más de una lista : el carril no pinta otra hasta
// que el raster coge la que hay (el ritmo lo marca Flutter).

class Relevo {

public:

	explicit Relevo(std :: size_t imagenes) ;

	// Hilo que pinta : una imagen libre (ya pintando), o -1 si no hay o si
	// ya hay una lista esperando a que el raster la coja
	int tomar_libre() ;

	// Si antes de escribir en ella hay que esperar el semáforo de GL (se
	// mostró y GL la soltó). Se consume : la siguiente vez, false
	bool consumir_espera(int imagen) ;

	// Hilo que pinta : la pintada pasa a lista
	void publicar(int imagen) ;

	// Hilo que pinta : no se pintó nada, vuelve libre como estaba
	void devolver(int imagen) ;

	// Raster : coge la lista, si hay. `mostrar` : la nueva a la vista, o -1
	// si no había; `soltar` : la que deja de mostrarse (GL debe avisar con su
	// semáforo), o -1
	struct Cambio {

		int mostrar = -1 ;

		int soltar = -1 ;

	} ;

	Cambio tomar_lista() ;

	// La que está a la vista, o -1
	int mostrada() const ;

	// Raster : si hay una lista que coger (antes de tomar_lista, para soltar
	// en GL la que está a la vista antes de que el carril pueda tomarla)
	bool hay_lista() const ;

	// Imágenes nuevas : todas libres, sin espera, ninguna a la vista
	void reiniciar() ;

private:

	enum class Estado { libre , pintando , lista , mostrada } ;

	struct Imagen {

		Estado estado = Estado :: libre ;

		bool espera = false ;

	} ;

	mutable std :: mutex mutex_ ;

	std :: vector<Imagen> imagenes_ ;

} ;
