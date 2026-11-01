# include "presentacion.h"

# include <epoxy/egl.h>

# include <epoxy/gl.h>

# include <gtk/gtk.h>

# include <atomic>

namespace {

// El glFlush de verdad (el despachador de GL), al que llama el nuestro
PFNGLFLUSHPROC flush_real = nullptr ;

// El hilo de GTK : sus glFlush no esperan (no son de Flutter)
GThread * hilo_gtk = nullptr ;

// Hasta cuándo (g_get_monotonic_time, µs) esperan los glFlush de Flutter :
// cada cambio de tamaño lo alarga
std :: atomic<gint64> esperar_hasta {0} ;

// Si la pantalla EGL de Flutter tiene vallas : -1 sin mirar, 0 no, 1 sí
std :: atomic<int> hay_vallas {-1} ;

// Lo que dura la espera tras el último cambio de tamaño : el fotograma del
// tamaño nuevo llega después del cambio, no a la vez
constexpr gint64 margen_us = 300000 ;

// Tope de una espera : con la GPU colgada, mejor un destello que la ventana
// parada
constexpr EGLTimeKHR tope_ns = 50000000 ;

void EPOXY_CALLSPEC flush_con_valla() {

	if (g_thread_self() == hilo_gtk || g_get_monotonic_time() > esperar_hasta.load(std :: memory_order_relaxed)) {

		flush_real() ;

		return ;

	}

	const EGLDisplay display = eglGetCurrentDisplay() ;

	if (display == EGL_NO_DISPLAY) {

		flush_real() ;

		return ;

	}

	int vallas = hay_vallas.load(std :: memory_order_relaxed) ;

	if (vallas < 0) {

		vallas = epoxy_has_egl_extension(display , "EGL_KHR_fence_sync") ? 1 : 0 ;

		hay_vallas.store(vallas , std :: memory_order_relaxed) ;

		if (!vallas) g_warning("sin EGL_KHR_fence_sync : el fotograma de Flutter no se sincroniza con GTK") ;

	}

	const EGLSyncKHR valla = vallas ? eglCreateSyncKHR(display , EGL_SYNC_FENCE_KHR , nullptr) : EGL_NO_SYNC_KHR ;

	flush_real() ;

	if (valla == EGL_NO_SYNC_KHR) return ;

	eglClientWaitSyncKHR(display , valla , 0 , tope_ns) ;

	eglDestroySyncKHR(display , valla) ;

}

void al_redimensionar(GtkWidget * , GdkRectangle * , gpointer) {

	esperar_hasta.store(g_get_monotonic_time() + margen_us , std :: memory_order_relaxed) ;

}

}

void sincronizar_presentacion(FlView * view) {

	if (g_strcmp0(g_getenv("NLTX_SINCRONIA") , "0") == 0) return ;

	flush_real = reinterpret_cast<PFNGLFLUSHPROC>(eglGetProcAddress("glFlush")) ;

	if (flush_real == nullptr) {

		g_warning("sin glFlush por eglGetProcAddress : el fotograma de Flutter no se sincroniza con GTK") ;

		return ;

	}

	hilo_gtk = g_thread_self() ;

	// el motor aún no ha arrancado : nadie más está leyendo este puntero
	glFlush = flush_con_valla ;

	g_signal_connect(view , "size-allocate" , G_CALLBACK(al_redimensionar) , nullptr) ;

}
