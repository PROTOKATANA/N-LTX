# pragma once

# include <cstddef>

# include <cstdint>

# include "recursos_lienzo.h"

// Lo que se ve del documento en este fotograma, en píxeles físicos

struct Recuadro {

	std :: uint32_t ancho ;

	std :: uint32_t alto ;

	float y0 ;

	float y1 ;

} ;

// Ejecuta las listas de un fotograma, con su pila de capas

class Ejecucion {

public:

	Ejecucion(const Gpu & gpu , Recursos & recursos , const Recuadro & recuadro) ;

	bool abrir_base(GError ** error) ;

	bool lista(rust :: Slice<const ltx :: Comando> comandos , Malla & malla , GError ** error) ;

	Diligent :: ITexture * resolver_base() ;

private:

	void fijar(std :: size_t i , const float * limpiar) ;

	void usar(const Pasada & pasada) ;

	void indices(const Pasada & pasada , const ltx :: Comando & c) ;

	void imagen(const ltx :: Comando & c) ;

	bool abrir_capa(GError ** error) ;

	void resolver(std :: size_t i) ;

	void cerrar_mascara() ;

	void cerrar_capa(const ltx :: Comando & c) ;

	Gpu gpu_ ;

	Recursos & r_ ;

	Recuadro recuadro_ ;

	std :: size_t profundidad_ = 0 ;

	Diligent :: ITexture * mascara_ = nullptr ;

	Diligent :: IPipelineState * actual_ = nullptr ;

} ;
