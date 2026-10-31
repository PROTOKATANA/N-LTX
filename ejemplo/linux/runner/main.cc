#include <dlfcn.h>

#include <cstdio>
#include <cstdlib>

#include "my_application.h"

int main(int argc, char** argv) {
  // GTK/libepoxy puede cargar GLX perezosamente en su primer draw incluso en
  // Wayland, y el constructor GLX de NVIDIA se interbloquea con el arranque de
  // Vulkan en el hilo raster de Flutter. Resolver GLX antes de arrancar los
  // hilos de Flutter lo evita (visto en CORTEX).
  (void)dlopen("libGLX.so.0", RTLD_NOW | RTLD_LOCAL);

  MyApplication* app = my_application_new();
  int status = g_application_run(G_APPLICATION(app), argc, argv);
  g_object_unref(app);

  // El EGL de NVIDIA se cae en su manejador atexit después de que Flutter
  // cierre el contexto que importó la textura Vulkan. GTK y Flutter ya
  // terminaron su apagado: el sistema libera el resto.
  std::fflush(nullptr);
  std::_Exit(status);
}
