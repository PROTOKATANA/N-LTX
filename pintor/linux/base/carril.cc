# include "carril.h"

# include "gpu_surface.h"

Carril :: Carril(std :: size_t indice) : indice_(indice) , hilo_([this] { bucle() ; }) {}

Carril :: ~Carril() {

	// con el candado : el hilo, o ve `parar_` al mirar la condición, o ya
	// duerme y le llega el aviso (sin él, el aviso podía perderse entre las
	// dos cosas y join() no volver)

	{

		std :: lock_guard<std :: mutex> lock(mutex_) ;

		parar_ = true ;

	}

	cv_.notify_all() ;

	// puede estar esperando al dispositivo

	gpu_despertar() ;

	if (hilo_.joinable()) hilo_.join() ;

}

std :: size_t Carril :: registrar(std :: function<void()> trabajo) {

	std :: lock_guard<std :: mutex> lock(mutex_) ;

	trabajos_.push_back(std :: move(trabajo)) ;

	pedidos_.push_back(false) ;

	return trabajos_.size() - 1 ;

}

void Carril :: pedir(std :: size_t trabajo) {

	{

		std :: lock_guard<std :: mutex> lock(mutex_) ;

		if (trabajo < pedidos_.size()) pedidos_[trabajo] = true ;

	}

	cv_.notify_all() ;

}

Diligent :: IRenderDevice * Carril :: device() const { return gpu_device() ; }

Diligent :: IDeviceContext * Carril :: context() const { return gpu_context(indice_) ; }

void Carril :: bucle() {

	// lo que se cree en este hilo es de este carril (ver gpu_mascara)

	const CarrilDelHilo propio(indice_) ;

	// sin dispositivo (no se pudo crear, o se apaga antes), nada que pintar :
	// el fallo lo cuenta el raster (ver GpuSurface :: mostrar)

	if (!gpu_esperar_dispositivo(parar_)) return ;

	while (true) {

		std :: vector<std :: size_t> toca ;

		std :: vector<std :: function<void()>> trabajos ;

		{

			std :: unique_lock<std :: mutex> lock(mutex_) ;

			cv_.wait(lock , [this] {

				if (parar_) return true ;

				for (const bool p : pedidos_) {

					if (p) return true ;

				}

				return false ;

			});

			if (parar_) return ;

			for (std :: size_t i = 0 ; i < pedidos_.size() ; i++) {

				if (pedidos_[i]) {

					pedidos_[i] = false ;

					toca.push_back(i) ;

				}

			}

			trabajos = trabajos_ ;

		}

		// fuera del candado : mientras pinta, se puede pedir otra vez

		for (const std :: size_t i : toca) trabajos[i]() ;

	}

}
