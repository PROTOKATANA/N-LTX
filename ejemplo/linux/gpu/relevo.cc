# include "relevo.h"

Relevo :: Relevo(std :: size_t imagenes) : imagenes_(imagenes) {}

namespace {

// Un índice de imagen fuera del relevo (-1 : ninguna tomada) no toca nada
bool valida(int imagen , std :: size_t total) { return imagen >= 0 && static_cast<std :: size_t>(imagen) < total ; }

}  // namespace

int Relevo :: tomar_libre() {

	std :: lock_guard<std :: mutex> lock(mutex_) ;

	for (const Imagen & i : imagenes_) {

		if (i.estado == Estado :: lista || i.estado == Estado :: pintando) return -1 ;

	}

	for (std :: size_t i = 0 ; i < imagenes_.size() ; i++) {

		if (imagenes_[i].estado == Estado :: libre) {

			imagenes_[i].estado = Estado :: pintando ;

			return static_cast<int>(i) ;

		}

	}

	return -1 ;

}

bool Relevo :: consumir_espera(int imagen) {

	std :: lock_guard<std :: mutex> lock(mutex_) ;

	if (!valida(imagen , imagenes_.size())) return false ;

	Imagen & i = imagenes_[static_cast<std :: size_t>(imagen)] ;

	const bool espera = i.espera ;

	i.espera = false ;

	return espera ;

}

void Relevo :: publicar(int imagen) {

	std :: lock_guard<std :: mutex> lock(mutex_) ;

	if (!valida(imagen , imagenes_.size())) return ;

	imagenes_[static_cast<std :: size_t>(imagen)].estado = Estado :: lista ;

}

void Relevo :: devolver(int imagen) {

	std :: lock_guard<std :: mutex> lock(mutex_) ;

	if (!valida(imagen , imagenes_.size())) return ;

	imagenes_[static_cast<std :: size_t>(imagen)].estado = Estado :: libre ;

}

Relevo :: Cambio Relevo :: tomar_lista() {

	std :: lock_guard<std :: mutex> lock(mutex_) ;

	Cambio c ;

	for (std :: size_t i = 0 ; i < imagenes_.size() ; i++) {

		if (imagenes_[i].estado == Estado :: lista) c.mostrar = static_cast<int>(i) ;

	}

	if (c.mostrar < 0) return c ;

	for (std :: size_t i = 0 ; i < imagenes_.size() ; i++) {

		if (imagenes_[i].estado == Estado :: mostrada) {

			imagenes_[i].estado = Estado :: libre ;

			imagenes_[i].espera = true ;

			c.soltar = static_cast<int>(i) ;

		}

	}

	imagenes_[static_cast<std :: size_t>(c.mostrar)].estado = Estado :: mostrada ;

	return c ;

}

int Relevo :: mostrada() const {

	std :: lock_guard<std :: mutex> lock(mutex_) ;

	for (std :: size_t i = 0 ; i < imagenes_.size() ; i++) {

		if (imagenes_[i].estado == Estado :: mostrada) return static_cast<int>(i) ;

	}

	return -1 ;

}

bool Relevo :: hay_lista() const {

	std :: lock_guard<std :: mutex> lock(mutex_) ;

	for (const Imagen & i : imagenes_) {

		if (i.estado == Estado :: lista) return true ;

	}

	return false ;

}

void Relevo :: reiniciar() {

	std :: lock_guard<std :: mutex> lock(mutex_) ;

	for (Imagen & i : imagenes_) i = Imagen{} ;

}
