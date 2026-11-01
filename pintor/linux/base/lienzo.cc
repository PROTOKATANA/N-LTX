# include "lienzo.h"

# include <gtk/gtk.h>

# include <algorithm>

Lienzo lienzo_de_monitores() {

	Lienzo l ;

	GdkDisplay * display = gdk_display_get_default() ;

	if (!display) return l ;

	for (int i = 0 ; i < gdk_display_get_n_monitors(display) ; ++i) {

		GdkMonitor * monitor = gdk_display_get_monitor(display , i) ;

		GdkRectangle geometria ;

		gdk_monitor_get_geometry(monitor , &geometria) ;

		const int escala = gdk_monitor_get_scale_factor(monitor) ;

		l.ancho = std :: max(l.ancho , static_cast<std :: uint32_t>(geometria.width * escala)) ;

		l.alto = std :: max(l.alto , static_cast<std :: uint32_t>(geometria.height * escala)) ;

	}

	l.ancho = std :: min(l.ancho , 4096u) ;

	l.alto = std :: min(l.alto , 4096u) ;

	return l ;

}
