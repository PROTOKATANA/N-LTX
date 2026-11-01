// Texturas GL de Flutter respaldadas por imágenes Vulkan de Diligent. Cada
// «familia» tiene su canal de método, su carril (su hilo de render), su
// fábrica de renderizadores y sus métodos propios; las pone la app.

# include "texturas.h"

# include <atomic>

# include <cstdint>

# include <cstring>

# include <unordered_map>

# include <utility>

# include "gpu_surface.h"

struct _LtxTexture {

	FlTextureGL parent_instance ;

	GpuRenderer * gpu ;

	std :: uint32_t id ;

	FlMethodChannel * channel ;

	std :: atomic<bool> * reported ;

	// Sigue registrada (hilo de plataforma) : tras «dispose» ya no se marca

	bool registrada ;

} ;

struct _LtxTextureClass {

	FlTextureGLClass parent_class ;

} ;

using LtxTexture = _LtxTexture ;

using LtxTextureClass = _LtxTextureClass ;

G_DEFINE_TYPE(LtxTexture , ltx_texture , fl_texture_gl_get_type())

namespace {

// A qué textura avisa un renderizador cuando termina un fotograma : la que lo
// tiene ahora (o ninguna, entre dispose y create). Solo la toca el hilo de
// plataforma; `en_cola`, el aviso ya va de camino (desde el carril)
struct Destino {

	LtxTexture * textura = nullptr ;

	std :: atomic<bool> en_cola{ false } ;

} ;

struct Familia {

	Familia(FamiliaTexturas def , std :: size_t indice) : def(std :: move(def)) , carril(std :: make_unique<Carril>(indice)) {}

	FamiliaTexturas def ;

	// su hilo de render

	std :: unique_ptr<Carril> carril ;

	FlMethodChannel * channel = nullptr ;

	std :: unordered_map<std :: uint32_t , LtxTexture *> textures ;

	// El pool retiene los renderizadores entre unregister/re-register; la
	// limpieza GL ocurre siempre en el hilo raster, nunca desde GTK.
	std :: vector<GpuRenderer *> libres ;

	std :: vector<std :: unique_ptr<GpuRenderer>> pool ;

	std :: unordered_map<GpuRenderer * , std :: shared_ptr<Destino>> destinos ;

} ;

FlTextureRegistrar * registrar = nullptr ;

std :: vector<Familia *> familias ;

struct ErrorNotice {

	FlMethodChannel * channel ;

	std :: uint32_t id ;

	char * message ;

} ;

gboolean notify_gpu_error(gpointer data) {

	auto * notice = static_cast<ErrorNotice *>(data) ;

	if (notice->channel) {

		g_autoptr(FlValue) value = fl_value_new_map() ;

		fl_value_set_string_take(value , "id" , fl_value_new_int(notice->id)) ;

		fl_value_set_string_take(value , "message" , fl_value_new_string(notice->message)) ;

		fl_method_channel_invoke_method(notice->channel , "rendererError" , value , nullptr , nullptr , nullptr) ;

	}

	// su referencia al canal (ver populate)

	g_clear_object(&notice->channel) ;

	g_free(notice->message) ;

	delete notice ;

	return G_SOURCE_REMOVE ;

}

// Hilo de plataforma : hay algo pendiente que dibujar en el próximo populate

void marcar(LtxTexture * texture) {

	if (!registrar || !texture->registrada) return ;

	texture->gpu->solicitar() ;

	fl_texture_registrar_mark_texture_frame_available(registrar , FL_TEXTURE(texture)) ;

}

// Hilo de plataforma : el carril terminó un fotograma (o hay imágenes que
// importar) : que Flutter vuelva a pintar la textura, sin pedir otro. El
// bucle de lo que se mueve solo lo cierra populate,
// que pide el siguiente al coger el nuevo : si la vista se oculta, populate
// no llega y el bucle se para solo; al volver, la primera marca de Dart lo
// arranca de nuevo

gboolean avisar_textura(gpointer data) {

	auto * destino = static_cast<std :: shared_ptr<Destino> *>(data) ;

	(* destino)->en_cola = false ;

	LtxTexture * texture = (* destino)->textura ;

	if (registrar && texture && texture->registrada) fl_texture_registrar_mark_texture_frame_available(registrar , FL_TEXTURE(texture)) ;

	delete destino ;

	return G_SOURCE_REMOVE ;

}

// Cualquier hilo (el carril) : el aviso, al hilo de plataforma, sin apilarlos

std :: function<void()> aviso_de(std :: shared_ptr<Destino> destino) {

	return [destino] {

		if (!destino->en_cola.exchange(true)) g_idle_add(avisar_textura , new std :: shared_ptr<Destino>(destino)) ;

	};

}

void reply_error(FlMethodCall * call , const char * message) {

	fl_method_call_respond_error(call , "native_texture" , message , nullptr , nullptr) ;

}

void reply_success(FlMethodCall * call , FlValue * value = nullptr) {

	fl_method_call_respond_success(call , value , nullptr) ;

}

// Un renderizador nuevo de la familia, en su pool, con su aviso; o nulo si
// la fábrica no puede
GpuRenderer * nuevo_renderizador(Familia * fam) {

	auto nuevo = fam->def.fabrica(* fam->carril) ;

	if (!nuevo) return nullptr ;

	fam->pool.push_back(std :: move(nuevo)) ;

	GpuRenderer * gpu = fam->pool.back().get() ;

	// a quién avisa al terminar un fotograma : una vez por renderizador

	auto destino = std :: make_shared<Destino>() ;

	fam->destinos.emplace(gpu , destino) ;

	gpu->avisar_con(aviso_de(destino)) ;

	return gpu ;

}

void crear(Familia * fam , std :: uint32_t id , FlMethodCall * call) {

	auto found = fam->textures.find(id) ;

	if (found == fam->textures.end()) {

		GpuRenderer * gpu = nullptr ;

		if (!fam->libres.empty()) {

			gpu = fam->libres.back() ;

			fam->libres.pop_back() ;

		} else {

			if (fam->pool.size() >= fam->def.maximo) {

				reply_error(call , "Máximo de superficies GPU simultáneas") ;

				return ;

			}

			gpu = nuevo_renderizador(fam) ;

			if (!gpu) {

				reply_error(call , "No se pudo crear el renderizador GPU") ;

				return ;

			}

		}

		gpu->reiniciar(id) ;

		auto * texture = reinterpret_cast<LtxTexture *>(g_object_new(ltx_texture_get_type() , nullptr)) ;

		texture->gpu = gpu ;

		texture->id = id ;

		// su propia referencia : el raster lo usa aunque la familia ya cerrara

		texture->channel = FL_METHOD_CHANNEL(g_object_ref(fam->channel)) ;

		if (!fl_texture_registrar_register_texture(registrar , FL_TEXTURE(texture))) {

			g_object_unref(texture) ;

			fam->libres.push_back(gpu) ;

			reply_error(call , "No se pudo registrar la textura GPU") ;

			return ;

		}

		texture->registrada = true ;

		fam->destinos[gpu]->textura = texture ;

		fam->textures.emplace(id , texture) ;

		found = fam->textures.find(id) ;

	}

	g_autoptr(FlValue) value = fl_value_new_int(fl_texture_get_id(FL_TEXTURE(found->second))) ;

	reply_success(call , value) ;

}

void soltar(Familia * fam , LtxTexture * texture) {

	texture->registrada = false ;

	// su renderizador ya no avisa a esta textura

	auto destino = fam->destinos.find(texture->gpu) ;

	if (destino != fam->destinos.end() && destino->second->textura == texture) destino->second->textura = nullptr ;

	fl_texture_registrar_unregister_texture(registrar , FL_TEXTURE(texture)) ;

	fam->libres.push_back(texture->gpu) ;

	g_object_unref(texture) ;

}

void handle_call(FlMethodChannel * , FlMethodCall * call , gpointer user_data) {

	auto * fam = static_cast<Familia *>(user_data) ;

	const char * name = fl_method_call_get_name(call) ;

	FlValue * args = fl_method_call_get_args(call) ;

	const std :: int64_t requested = args && fl_value_get_type(args) == FL_VALUE_TYPE_INT ? fl_value_get_int(args) : 0 ;

	if (requested < 0 || requested > UINT32_MAX) {

		reply_error(call , "id inválido") ;

		return ;

	}

	const auto id = static_cast<std :: uint32_t>(requested) ;

	auto found = fam->textures.find(id) ;

	LtxTexture * texture = found != fam->textures.end() ? found->second : nullptr ;

	if (std :: strcmp(name , "create") == 0) {

		crear(fam , id , call) ;

	} else if (std :: strcmp(name , "mark") == 0) {

		if (texture) marcar(texture) ;

		reply_success(call) ;

	} else if (std :: strcmp(name , "dispose") == 0) {

		if (texture) {

			soltar(fam , texture) ;

			fam->textures.erase(found) ;

		}

		reply_success(call) ;

	} else {

		LlamadaTextura llamada{ name , args , texture ? texture->gpu : nullptr , call , [texture]() { if (texture) marcar(texture) ; } } ;

		if (!fam->def.metodo || !fam->def.metodo(llamada)) fl_method_call_respond_not_implemented(call , nullptr) ;

	}

}

}  // namespace

static gboolean populate(

	FlTextureGL * base ,

	std :: uint32_t * target ,

	std :: uint32_t * name ,

	std :: uint32_t * width ,

	std :: uint32_t * height ,

	GError ** error

){

	auto * texture = reinterpret_cast<LtxTexture *>(base) ;

	// la última imagen terminada en el carril : populate nunca espera a la GPU

	if (texture->gpu && texture->gpu->texture_info(target , name , width , height , error)) return TRUE ;

	if (error && !* error)
		g_set_error_literal(error , G_IO_ERROR , G_IO_ERROR_FAILED , "PRESENTATION_FAILED: escena no disponible") ;

	if (error && * error && !texture->reported->exchange(true)) {

		// el aviso lleva su propia referencia al canal : puede llegar al hilo de
		// plataforma después de soltar la textura

		g_main_context_invoke(

			nullptr ,

			notify_gpu_error ,

			new ErrorNotice{ texture->channel ? FL_METHOD_CHANNEL(g_object_ref(texture->channel)) : nullptr , texture->id , g_strdup((* error)->message) }

		) ;

	}

	return FALSE ;

}

static void finalize_texture(GObject * object) {

	auto * texture = reinterpret_cast<LtxTexture *>(object) ;

	g_clear_object(&texture->channel) ;

	delete texture->reported ;

	G_OBJECT_CLASS(ltx_texture_parent_class)->finalize(object) ;

}

static void ltx_texture_class_init(LtxTextureClass * klass) {

	FL_TEXTURE_GL_CLASS(klass)->populate = populate ;

	G_OBJECT_CLASS(klass)->finalize = finalize_texture ;

}

static void ltx_texture_init(LtxTexture * texture) {

	texture->gpu = nullptr ;

	texture->channel = nullptr ;

	texture->reported = new std :: atomic<bool>(false) ;

	texture->registrada = false ;

}

void registrar_texturas(FlView * view , std :: vector<FamiliaTexturas> definiciones) {

	g_autoptr(FlPluginRegistrar) plugin = fl_plugin_registry_get_registrar_for_plugin(FL_PLUGIN_REGISTRY(view) , "CnxTexturas") ;

	registrar = fl_plugin_registrar_get_texture_registrar(plugin) ;

	// un carril (y un contexto del dispositivo) por familia

	gpu_carriles(definiciones.size()) ;

	for (FamiliaTexturas & def : definiciones) {

		auto * fam = new Familia(std :: move(def) , familias.size()) ;

		g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new() ;

		fam->channel = fl_method_channel_new(fl_plugin_registrar_get_messenger(plugin) , fam->def.canal , FL_METHOD_CODEC(codec)) ;

		fl_method_channel_set_method_call_handler(fam->channel , handle_call , fam , nullptr) ;

		// listo antes de que Flutter lo pida : su carril prepara mientras la
		// app arranca

		if (fam->def.precalentar) {

			if (GpuRenderer * gpu = nuevo_renderizador(fam)) {

				fam->libres.push_back(gpu) ;

				gpu->precalentar() ;

			}

		}

		familias.push_back(fam) ;

	}

}

void cerrar_texturas() {

	for (Familia * fam : familias) {

		// primero, que el carril deje de pintar (termina su fotograma y para)

		fam->carril.reset() ;

		for (auto & item : fam->textures) soltar(fam , item.second) ;

		fam->textures.clear() ;

		fam->libres.clear() ;

		// Los handles externos del pool los libera el propio proceso al salir; no
		// hacer llamadas GL desde GTK durante el teardown del embedder.
		for (auto & target : fam->pool) target.release() ;

		fam->pool.clear() ;

		// el canal puede sobrevivir a la familia (lo retienen sus texturas y los
		// avisos pendientes) : sin manejador, una llamada tardía de Dart no
		// llega a la familia ya borrada

		fl_method_channel_set_method_call_handler(fam->channel , nullptr , nullptr , nullptr) ;

		g_clear_object(&fam->channel) ;

		if (fam->def.cerrar) fam->def.cerrar() ;

		delete fam ;

	}

	familias.clear() ;

	registrar = nullptr ;

}
