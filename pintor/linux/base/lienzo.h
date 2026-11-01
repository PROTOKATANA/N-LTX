# pragma once

# include <cstdint>

// El lienzo de las texturas : el monitor más grande en px físicos (con tope
// de 4096 por lado). Una textura de ese tamaño no se recrea al redimensionar
// la ventana; el renderizador pinta solo la parte que se ve. Hilo de
// plataforma, tras fl_register_plugins (pregunta a GDK); sin pantalla, 0 × 0
struct Lienzo {

	std :: uint32_t ancho = 0 ;

	std :: uint32_t alto = 0 ;

} ;

Lienzo lienzo_de_monitores() ;
