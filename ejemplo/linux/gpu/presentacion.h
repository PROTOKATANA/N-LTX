# pragma once

# include <flutter_linux/flutter_linux.h>

// Que GTK no enseñe un fotograma de Flutter sin terminar al redimensionar.
//
// Flutter pinta la ventana en su contexto GL (el raster) y GTK la lee desde
// el suyo a través de una EGLImage, sin más sincronía que un glFlush : el
// trabajo está enviado, no hecho. Al cambiar el tamaño, Flutter crea un búfer
// nuevo (vacío, negro); si la GPU va cargada, GTK lo lee antes
// de que Flutter lo haya pintado y la ventana entera destella en negro.
//
// El arreglo, sin tocar Flutter : mientras dura un redimensionado (y un rato
// después), cada glFlush de los hilos de Flutter deja una valla EGL y espera
// a que la GPU la pase, así que cuando GTK lee el fotograma ya está pintado.
// Fuera del redimensionado, glFlush es el de siempre. NLTX_SINCRONIA=0 lo
// apaga (para comparar).
//
// Hilo de plataforma, antes de que arranque el motor (antes de realizar la
// vista) : cambia el glFlush de epoxy para todo el proceso
void sincronizar_presentacion(FlView * view) ;
