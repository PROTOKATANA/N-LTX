# pragma once

# include <cstddef>

# include <cstdint>

# include <string>

# include <vector>

# include "DeviceContext.h"

# include "RenderDevice.h"

// Los bancos de RAYO : el fotograma entero y cada etapa de cada pasada, en
// CPU (lo que gasta el hilo del carril, sin contar esperas ni el tiempo sin
// núcleo) y en GPU (consultas de duración). Cada 300 fotogramas, medianas y
// p95. Espera a la GPU al final de cada fotograma : solo para medir, nunca en
// la app normal. Cada renderizador lo crea con el prefijo de sus mensajes

enum class Etapa { preparar , subir , dibujar } ;

class BancoGpu {

public:

	// `prefijo` : el de sus mensajes («LTX»); `pasadas` : el
	// nombre de cada una, en el orden en que se dibujan

	BancoGpu(std :: string prefijo , std :: vector<std :: string> pasadas) ;

	~BancoGpu() ;

	BancoGpu(const BancoGpu &) = delete ;

	BancoGpu & operator=(const BancoGpu &) = delete ;

	// Al empezar el fotograma, y justo antes de pedir el turno a EGL

	void empezar() ;

	void comenzar() ;

	// El principio y el final de una etapa de una pasada; con contexto
	// (subir y dibujar), también en la GPU

	void abrir(std :: size_t pasada , Etapa etapa , Diligent :: IRenderDevice * device , Diligent :: IDeviceContext * ctx) ;

	void cerrar(std :: size_t pasada , Etapa etapa , Diligent :: IDeviceContext * ctx) ;

	// Tras devolver el turno : espera a la GPU, lee las consultas y apunta

	void terminar(Diligent :: IDeviceContext * ctx , std :: uint32_t ancho , std :: uint32_t alto) ;

private:

	struct Impl ;

	Impl * impl_ ;

} ;

// Una etapa medida : se abre al crearse y se cierra al salir del ámbito (también
// en un return temprano). Sin banco no hace nada

class Tramo {

public:

	Tramo(

		BancoGpu * banco ,

		std :: size_t pasada ,

		Etapa etapa ,

		Diligent :: IRenderDevice * device = nullptr ,

		Diligent :: IDeviceContext * ctx = nullptr

	) ;

	~Tramo() ;

	Tramo(const Tramo &) = delete ;

	Tramo & operator=(const Tramo &) = delete ;

private:

	BancoGpu * banco_ ;

	std :: size_t pasada_ ;

	Etapa etapa_ ;

	Diligent :: IDeviceContext * ctx_ ;

} ;
