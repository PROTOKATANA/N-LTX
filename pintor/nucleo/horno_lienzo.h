# pragma once

# include <string>

# include "RefCntAutoPtr.hpp"

# include "RenderDevice.h"

# include "Shader.h"

// Convertir los shaders del lienzo en objetos de Diligent : de lo horneado si se
// puede y del HLSL si no. `hornear`, que genera lo horneado, está en pintor_lienzo.h

namespace ltx {

// `fuente` con los tipos de tinta delante, como los dice Rust

std :: string con_tipos(const char * fuente) ;

// Compilar HLSL en cada arranque se llevaba 32 de los 36 ms de preparar el pintor,
// y no depende de ningún documento : en Vulkan se usa lo horneado. Si el texto
// cambia, su huella ya no está y se compila como antes. Otros backends compilan
// siempre

Diligent :: RefCntAutoPtr<Diligent :: IShader> compilar(Diligent :: IRenderDevice * device , const char * nombre , Diligent :: SHADER_TYPE tipo , const char * fuente) ;

}  // namespace ltx
