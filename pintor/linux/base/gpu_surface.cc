// Superficie compartida Vulkan↔EGL, en dos hilos (el carril y el raster).
// El esquema de interop : dispositivo Vulkan vía Diligent, imágenes y
// semáforos exportados a EGL.

# include "gpu_surface.h"

# include <dlfcn.h>

# include <unistd.h>

# include <epoxy/egl.h>

# include <epoxy/gl.h>

# include <vulkan/vulkan.h>

# include <algorithm>

# include <condition_variable>

# include <cstdio>

# include <cstdlib>

# include <cstring>

# include <mutex>

# include <string>

# include <unordered_map>

# include <vector>

# include "CommandQueueVk.h"

# include "DeviceContextVk.h"

# include "EngineFactoryVk.h"

# include "RenderDeviceVk.h"

# include "relevo.h"

using namespace Diligent ;

namespace {

// Estructura de Vulkan a cero con su sType (y pNext): el patrón habitual sin
// inicializadores parciales, que -Wextra señala.
template <typename T>
T vk_struct(VkStructureType tipo , decltype(T :: pNext) siguiente = nullptr) {

	T s{} ;

	s.sType = tipo ;

	s.pNext = siguiente ;

	return s ;

}

// Diligent usa un loader Vulkan privado (volk); las funciones de interop se
// cargan a mano del loader del sistema contra su instancia/dispositivo.
struct VulkanApi {

	void * library = nullptr ;

	PFN_vkGetInstanceProcAddr get_instance = nullptr ;

	PFN_vkGetDeviceProcAddr get_device = nullptr ;

	PFN_vkGetPhysicalDeviceProperties2 GetPhysicalDeviceProperties2 = nullptr ;

	PFN_vkGetPhysicalDeviceQueueFamilyProperties GetPhysicalDeviceQueueFamilyProperties = nullptr ;

	PFN_vkGetPhysicalDeviceMemoryProperties GetPhysicalDeviceMemoryProperties = nullptr ;

	PFN_vkGetDeviceQueue GetDeviceQueue = nullptr ;

	PFN_vkCreateImage CreateImage = nullptr ;

	PFN_vkGetImageMemoryRequirements GetImageMemoryRequirements = nullptr ;

	PFN_vkAllocateMemory AllocateMemory = nullptr ;

	PFN_vkBindImageMemory BindImageMemory = nullptr ;

	PFN_vkCreateSemaphore CreateSemaphore = nullptr ;

	PFN_vkQueueSubmit QueueSubmit = nullptr ;

	PFN_vkDeviceWaitIdle DeviceWaitIdle = nullptr ;

	PFN_vkDestroySemaphore DestroySemaphore = nullptr ;

	PFN_vkDestroyImage DestroyImage = nullptr ;

	PFN_vkFreeMemory FreeMemory = nullptr ;

	PFN_vkGetMemoryFdKHR GetMemoryFdKHR = nullptr ;

	PFN_vkGetSemaphoreFdKHR GetSemaphoreFdKHR = nullptr ;

	bool load(VkInstance instance , VkDevice device) {

		library = dlopen("libvulkan.so.1" , RTLD_NOW | RTLD_LOCAL) ;

		if (!library) return false ;

		get_instance = reinterpret_cast<PFN_vkGetInstanceProcAddr>(dlsym(library , "vkGetInstanceProcAddr")) ;

		if (!get_instance) return false ;

		get_device = reinterpret_cast<PFN_vkGetDeviceProcAddr>(get_instance(instance , "vkGetDeviceProcAddr")) ;

		if (!get_device) return false ;

		GetPhysicalDeviceProperties2 = reinterpret_cast<PFN_vkGetPhysicalDeviceProperties2>(

			get_instance(instance , "vkGetPhysicalDeviceProperties2")

		) ;

		if (!GetPhysicalDeviceProperties2)
			GetPhysicalDeviceProperties2 = reinterpret_cast<PFN_vkGetPhysicalDeviceProperties2>(

				get_instance(instance , "vkGetPhysicalDeviceProperties2KHR")

			) ;

# define LOAD_INSTANCE(member , symbol) \
	member = reinterpret_cast<decltype(member)>(get_instance(instance , symbol))

# define LOAD_DEVICE(member , symbol) \
	member = reinterpret_cast<decltype(member)>(get_device(device , symbol))

		LOAD_INSTANCE(GetPhysicalDeviceQueueFamilyProperties , "vkGetPhysicalDeviceQueueFamilyProperties") ;

		LOAD_INSTANCE(GetPhysicalDeviceMemoryProperties , "vkGetPhysicalDeviceMemoryProperties") ;

		LOAD_DEVICE(GetDeviceQueue , "vkGetDeviceQueue") ;

		LOAD_DEVICE(CreateImage , "vkCreateImage") ;

		LOAD_DEVICE(GetImageMemoryRequirements , "vkGetImageMemoryRequirements") ;

		LOAD_DEVICE(AllocateMemory , "vkAllocateMemory") ;

		LOAD_DEVICE(BindImageMemory , "vkBindImageMemory") ;

		LOAD_DEVICE(CreateSemaphore , "vkCreateSemaphore") ;

		LOAD_DEVICE(QueueSubmit , "vkQueueSubmit") ;

		LOAD_DEVICE(DeviceWaitIdle , "vkDeviceWaitIdle") ;

		LOAD_DEVICE(DestroySemaphore , "vkDestroySemaphore") ;

		LOAD_DEVICE(DestroyImage , "vkDestroyImage") ;

		LOAD_DEVICE(FreeMemory , "vkFreeMemory") ;

		LOAD_DEVICE(GetMemoryFdKHR , "vkGetMemoryFdKHR") ;

		LOAD_DEVICE(GetSemaphoreFdKHR , "vkGetSemaphoreFdKHR") ;

# undef LOAD_INSTANCE

# undef LOAD_DEVICE

		return GetPhysicalDeviceProperties2
			&& GetPhysicalDeviceQueueFamilyProperties
			&& GetPhysicalDeviceMemoryProperties
			&& GetDeviceQueue
			&& CreateImage
			&& GetImageMemoryRequirements
			&& AllocateMemory
			&& BindImageMemory
			&& CreateSemaphore
			&& QueueSubmit
			&& DeviceWaitIdle
			&& DestroySemaphore
			&& DestroyImage
			&& FreeMemory
			&& GetMemoryFdKHR
			&& GetSemaphoreFdKHR ;

	}

} ;

bool check_vk(VkResult result , GError ** error , const char * operation) {

	if (result == VK_SUCCESS) return true ;

	g_set_error(error , G_IO_ERROR , G_IO_ERROR_FAILED , "%s: Vulkan devolvió %d" , operation , result) ;

	return false ;

}

// Los mensajes de Diligent : sin los informativos (las capas y extensiones de
// Vulkan, la memoria…, un centenar de líneas al arrancar); sus avisos y
// errores, sí
void DILIGENT_CALL_TYPE mensaje_diligent(DEBUG_MESSAGE_SEVERITY gravedad , const Char * mensaje , const Char * , const Char * , int) {

	if (gravedad != DEBUG_MESSAGE_SEVERITY_INFO && mensaje) g_warning("Diligent: %s" , mensaje) ;

}


// El dispositivo de la base : uno, con un contexto inmediato por carril. Lo
// crea el primer carril que arranca, cuando el raster ya leyó la GPU de GL
// (su UUID : Vulkan tiene que usar la misma). `estados` va el último : los
// pipelines de los renderizadores se liberan antes que el dispositivo.
struct GpuDevice {

	std :: mutex mutex ;

	std :: condition_variable cambio ;

	// la GPU de GL (la lee el raster) y la de Vulkan (al crearlo) : deben ser
	// la misma; lo comprueba quien llega segundo

	bool gl_leido = false ;

	GLubyte gl_uuid[VK_UUID_SIZE]{} ;

	GLubyte vk_uuid[VK_UUID_SIZE]{} ;

	std :: size_t carriles = 1 ;

	bool intentado = false ;

	bool listo = false ;

	std :: string fallo ;

	RefCntAutoPtr<IRenderDevice> device ;

	RefCntAutoPtr<IRenderDeviceVk> device_vk ;

	std :: vector<RefCntAutoPtr<IDeviceContext>> contexts ;

	std :: vector<RefCntAutoPtr<IDeviceContextVk>> contexts_vk ;

	VkDevice vk_device = VK_NULL_HANDLE ;

	VulkanApi vk ;

	VkPhysicalDevice vk_physical = VK_NULL_HANDLE ;

	// ReleaseStaleResources, desde varios carriles

	std :: mutex liberar ;

	std :: mutex estados_mutex ;

	std :: unordered_map<const void * , std :: shared_ptr<void>> estados ;

} ;

GpuDevice & dispositivo() {

	static GpuDevice d ;

	return d ;

}

constexpr const char * gpus_distintas = "EGL y Vulkan seleccionaron GPUs distintas" ;

// Hilo raster, la primera vez : lo que Vulkan necesita de GL (la memoria y
// los semáforos externos) y que su GPU sea la de Vulkan. El dispositivo no lo
// espera : se crea al arrancar la app, y la UUID se compara aquí o al crearlo
bool leer_gl(GError ** error) {

	auto & d = dispositivo() ;

	{

		std :: lock_guard<std :: mutex> lock(d.mutex) ;

		if (d.gl_leido) return true ;

	}

	if (!glGetString(GL_VERSION)) return gpu_fallo(error , "Flutter no activó EGL") ;

	if (!epoxy_has_gl_extension("GL_EXT_memory_object_fd") || !epoxy_has_gl_extension("GL_EXT_semaphore_fd"))
		return gpu_fallo(error , "La GPU EGL no admite memoria o semáforos externos") ;

	GLint uuid_count = 0 ;

	glGetIntegerv(GL_NUM_DEVICE_UUIDS_EXT , &uuid_count) ;

	if (uuid_count < 1) return gpu_fallo(error , "EGL no expone la UUID de su GPU") ;

	GLubyte uuid[VK_UUID_SIZE]{} ;

	glGetUnsignedBytei_vEXT(GL_DEVICE_UUID_EXT , 0 , uuid) ;

	{

		std :: lock_guard<std :: mutex> lock(d.mutex) ;

		std :: memcpy(d.gl_uuid , uuid , VK_UUID_SIZE) ;

		d.gl_leido = true ;

		if (d.listo && std :: memcmp(d.vk_uuid , d.gl_uuid , VK_UUID_SIZE) != 0) d.fallo = gpus_distintas ;

	}

	return true ;

}

// Hilo del carril (el primero que llega) : el dispositivo, con un contexto
// por carril en la cola gráfica del adaptador que elegiría Diligent (la
// primera GPU dedicada con cola gráfica). Devuelve el fallo, o vacío
std :: string crear_dispositivo(GpuDevice & d , GLubyte vk_uuid[VK_UUID_SIZE] , std :: size_t carriles) {

	IEngineFactoryVk * fabrica = GetEngineFactoryVk() ;

	fabrica->SetMessageCallback(mensaje_diligent) ;

	const char * extensions[] = { VK_KHR_EXTERNAL_MEMORY_FD_EXTENSION_NAME , VK_KHR_EXTERNAL_SEMAPHORE_FD_EXTENSION_NAME } ;

	EngineVkCreateInfo ci ;

	// el banco (NLTX_BANCO_GPU) mide cada pasada con
	// consultas de duración; si la GPU no las tiene, no pasa nada

	ci.Features.DurationQueries = DEVICE_FEATURE_STATE_OPTIONAL ;

	// RAYO, no guardar lo que ya no hace falta : Diligent conserva hasta 256 MiB
	// de páginas visibles desde la CPU ya liberadas, y la subida de los buffers
	// inmutables al arrancar las dejaba residentes. Con una página de reserva
	// (la del cbuffer de cada fotograma sigue viva) se devuelven al driver

	ci.HostVisibleMemoryReserveSize = ci.HostVisibleMemoryPageSize ;

	ci.DeviceExtensionCount = 2 ;

	ci.ppDeviceExtensionNames = extensions ;

	// el adaptador y su cola gráfica, con sitio para todos los carriles

	Uint32 cuantos = 0 ;

	fabrica->EnumerateAdapters(ci.GraphicsAPIVersion , cuantos , nullptr) ;

	std :: vector<GraphicsAdapterInfo> adaptadores(cuantos) ;

	if (cuantos) fabrica->EnumerateAdapters(ci.GraphicsAPIVersion , cuantos , adaptadores.data()) ;

	int elegido = -1 , cola = -1 ;

	for (Uint32 i = 0 ; i < cuantos ; i++) {

		const GraphicsAdapterInfo & a = adaptadores[i] ;

		int q = -1 ;

		for (Uint32 k = 0 ; k < a.NumQueues && q < 0 ; k++) {

			if ((a.Queues[k].QueueType & COMMAND_QUEUE_TYPE_GRAPHICS) == COMMAND_QUEUE_TYPE_GRAPHICS && a.Queues[k].MaxDeviceContexts >= carriles) q = static_cast<int>(k) ;

		}

		if (q < 0) continue ;

		elegido = static_cast<int>(i) ;

		cola = q ;

		if (a.Type == ADAPTER_TYPE_DISCRETE) break ;

	}

	if (elegido < 0) return "No hay GPU con cola gráfica para todos los carriles" ;

	ci.AdapterId = static_cast<Uint32>(elegido) ;

	std :: vector<std :: string> nombres(carriles) ;

	std :: vector<ImmediateContextCreateInfo> contextos(carriles) ;

	// Prioridad baja en la GPU (VK_EXT_global_priority) : se le pide al driver
	// que, con la GPU cargada, la interfaz vaya antes que los shaders. Todos
	// los contextos de una familia deben tener la misma

	const QUEUE_PRIORITY prioridad = QUEUE_PRIORITY_LOW ;

	for (std :: size_t i = 0 ; i < carriles ; i++) {

		nombres[i] = "N-LTX carril " + std :: to_string(i) ;

		contextos[i].Name = nombres[i].c_str() ;

		contextos[i].QueueId = static_cast<Uint8>(cola) ;

		contextos[i].Priority = prioridad ;

	}

	ci.NumImmediateContexts = static_cast<Uint32>(carriles) ;

	ci.pImmediateContextInfo = contextos.data() ;

	std :: vector<IDeviceContext *> crudos(carriles , nullptr) ;

	fabrica->CreateDeviceAndContextsVk(ci , &d.device , crudos.data()) ;

	// si el driver no admite la baja, con la de siempre antes que sin GPU

	if (!d.device && prioridad != QUEUE_PRIORITY_MEDIUM) {

		for (ImmediateContextCreateInfo & c : contextos) c.Priority = QUEUE_PRIORITY_MEDIUM ;

		std :: fill(crudos.begin() , crudos.end() , nullptr) ;

		fabrica->CreateDeviceAndContextsVk(ci , &d.device , crudos.data()) ;

	}

	d.contexts.resize(carriles) ;

	d.contexts_vk.resize(carriles) ;

	for (std :: size_t i = 0 ; i < carriles ; i++) d.contexts[i].Attach(crudos[i]) ;

	if (!d.device) return "Diligent no pudo iniciar Vulkan" ;

	d.device_vk = RefCntAutoPtr<IRenderDeviceVk>(d.device , IID_RenderDeviceVk) ;

	if (!d.device_vk) return "Falta el adaptador Vulkan de Diligent" ;

	for (std :: size_t i = 0 ; i < carriles ; i++) {

		if (!d.contexts[i]) return "Diligent no pudo crear los contextos de los carriles" ;

		d.contexts_vk[i] = RefCntAutoPtr<IDeviceContextVk>(d.contexts[i] , IID_DeviceContextVk) ;

		if (!d.contexts_vk[i]) return "Falta el adaptador Vulkan de Diligent" ;

	}

	d.vk_device = d.device_vk->GetVkDevice() ;

	d.vk_physical = d.device_vk->GetVkPhysicalDevice() ;

	if (!d.vk.load(d.device_vk->GetVkInstance() , d.vk_device)) return "No se pudieron cargar las funciones Vulkan externas" ;

	auto ids = vk_struct<VkPhysicalDeviceIDProperties>(VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_ID_PROPERTIES) ;

	auto props = vk_struct<VkPhysicalDeviceProperties2>(VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2 , &ids) ;

	d.vk.GetPhysicalDeviceProperties2(d.vk_physical , &props) ;

	std :: memcpy(vk_uuid , ids.deviceUUID , VK_UUID_SIZE) ;

	return "" ;

}

}  // namespace

void gpu_carriles(std :: size_t carriles) {

	auto & d = dispositivo() ;

	std :: lock_guard<std :: mutex> lock(d.mutex) ;

	d.carriles = std :: max<std :: size_t>(carriles , 1) ;

}

bool gpu_esperar_dispositivo(const std :: atomic<bool> & parar) {

	auto & d = dispositivo() ;

	std :: unique_lock<std :: mutex> lock(d.mutex) ;

	if (parar) return false ;

	// al arrancar la app, sin esperar a GL : los carriles preparan sus shaders
	// mientras Flutter arranca (la GPU de GL se compara cuando se lea)

	if (!d.intentado) {

		d.intentado = true ;

		const std :: size_t carriles = d.carriles ;

		// crearlo tarda : sin el candado, los demás carriles esperan abajo

		lock.unlock() ;

		GLubyte uuid[VK_UUID_SIZE]{} ;

		std :: string fallo = crear_dispositivo(d , uuid , carriles) ;

		lock.lock() ;

		std :: memcpy(d.vk_uuid , uuid , VK_UUID_SIZE) ;

		if (fallo.empty() && d.gl_leido && std :: memcmp(d.vk_uuid , d.gl_uuid , VK_UUID_SIZE) != 0) fallo = gpus_distintas ;

		if (fallo.empty()) { d.listo = true ; }

		else { d.fallo = std :: move(fallo) ; }

		d.cambio.notify_all() ;

	}

	d.cambio.wait(lock , [&] { return parar || d.listo || !d.fallo.empty() ; }) ;

	return d.listo && !parar ;

}

void gpu_despertar() {

	auto & d = dispositivo() ;

	// tomar el candado tras cambiar `parar` y antes de avisar : quien espera,
	// o ve el cambio al mirar la condición, o ya duerme y le llega el aviso

	{ const std :: lock_guard<std :: mutex> lock(d.mutex) ; }

	d.cambio.notify_all() ;

}

IRenderDevice * gpu_device() { return dispositivo().device ; }

IDeviceContext * gpu_context(std :: size_t carril) {

	auto & d = dispositivo() ;

	return carril < d.contexts.size() ? d.contexts[carril].RawPtr() : nullptr ;

}

namespace {

thread_local std :: size_t carril_del_hilo = SIZE_MAX ;

}  // namespace

std :: size_t gpu_carril_del_hilo() { return carril_del_hilo ; }

CarrilDelHilo :: CarrilDelHilo(std :: size_t carril) : antes_(carril_del_hilo) { carril_del_hilo = carril ; }

CarrilDelHilo :: ~CarrilDelHilo() { carril_del_hilo = antes_ ; }

Uint64 gpu_mascara() {

	if (carril_del_hilo < 64) return Uint64{ 1 } << carril_del_hilo ;

	static std :: atomic<bool> dicho{ false } ;

	if (!dicho.exchange(true)) g_critical("N-LTX GPU: un recurso creado fuera de un carril; se marca para todos (se liberará más tarde)") ;

	auto & d = dispositivo() ;

	return d.carriles >= 64 ? ~Uint64{ 0 } : (Uint64{ 1 } << d.carriles) - 1 ;

}

bool gpu_fallo(GError ** error , const char * mensaje) {

	g_set_error_literal(error , G_IO_ERROR , G_IO_ERROR_FAILED , mensaje) ;

	return false ;

}

bool gpu_asegurar_buffer(

	IRenderDevice * device ,

	RefCntAutoPtr<IBuffer> & buffer ,

	std :: uint64_t & capacidad ,

	std :: uint64_t necesario ,

	BIND_FLAGS flags ,

	const char * nombre ,

	GError ** error

){

	if (buffer && capacidad >= necesario) return true ;

	std :: uint64_t nueva = std :: max<std :: uint64_t>(necesario * 3 / 2 , 1 << 16) ;

	BufferDesc desc ;

	desc.Name = nombre ;

	desc.Size = nueva ;

	desc.Usage = USAGE_DEFAULT ;

	desc.BindFlags = flags ;

	buffer.Release() ;

	desc.ImmediateContextMask = gpu_mascara() ;

	device->CreateBuffer(desc , nullptr , &buffer) ;

	if (!buffer) return gpu_fallo(error , "OUT_OF_GPU_MEMORY: buffer de GPU") ;

	capacidad = nueva ;

	return true ;

}

void gpu_color(std :: uint32_t rgba , float out[4]) {

	out[0] = ((rgba >> 24) & 255) / 255.0f ;

	out[1] = ((rgba >> 16) & 255) / 255.0f ;

	out[2] = ((rgba >> 8) & 255) / 255.0f ;

	out[3] = (rgba & 255) / 255.0f ;

}

namespace {

// Una de las imágenes que se relevan : la VkImage con su memoria y sus dos
// semáforos (exportados), lo que Diligent ve de ella (del carril), y lo mismo
// importado en GL (del raster)
struct Imagen {

	RefCntAutoPtr<ITexture> textura ;

	VkImage image = VK_NULL_HANDLE ;

	VkDeviceMemory memory = VK_NULL_HANDLE ;

	VkDeviceSize bytes = 0 ;

	// Vulkan → GL : la terminó; GL → Vulkan : la soltó

	VkSemaphore vk_ready = VK_NULL_HANDLE ;

	VkSemaphore gl_ready = VK_NULL_HANDLE ;

	// exportados, por importar en GL (GL se los queda)

	int fd_memoria = -1 , fd_vk = -1 , fd_gl = -1 ;

	GLuint gl_texture = 0 , gl_memory = 0 , gl_vk_ready = 0 , gl_gl_ready = 0 ;

	// el volcado pendiente de este fotograma (lo hace el raster al mostrarla)

	std :: string volcado ;

	std :: uint32_t volcado_w = 0 , volcado_h = 0 ;

	// la región que se pintó en ella la última vez (fijar_region) : fuera,
	// siempre el fondo

	std :: uint32_t region_w = 0 , region_h = 0 ;

} ;

// Las imágenes de la superficie, entre el carril y el raster :
//   sin_imagenes → (carril las crea) por_importar → (raster las importa) lista
//   lista → (otro tamaño) por_retirar → (raster las suelta) retirada →
//   (carril las destruye y crea otras) por_importar …
enum class Fase { sin_imagenes , por_importar , lista , por_retirar , retirada } ;

constexpr std :: size_t total_imagenes = 2 ;

void cerrar_fd(int & fd) {

	if (fd >= 0) close(fd) ;

	fd = -1 ;

}

// Hilo del carril : la imagen en Vulkan, con su memoria y sus semáforos
// exportados, y su textura de Diligent
bool crear_vk(GpuDevice & dev , const std :: string & nombre , Imagen & im , std :: uint32_t w , std :: uint32_t h , GError ** error) {

	auto external_image = vk_struct<VkExternalMemoryImageCreateInfo>(VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO) ;

	external_image.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT ;

	auto image_ci = vk_struct<VkImageCreateInfo>(VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO , &external_image) ;

	image_ci.imageType = VK_IMAGE_TYPE_2D ;

	image_ci.format = VK_FORMAT_R8G8B8A8_UNORM ;

	image_ci.extent = { w , h , 1 } ;

	image_ci.mipLevels = image_ci.arrayLayers = 1 ;

	image_ci.samples = VK_SAMPLE_COUNT_1_BIT ;

	image_ci.tiling = VK_IMAGE_TILING_OPTIMAL ;

	image_ci.usage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_SAMPLED_BIT ;

	image_ci.sharingMode = VK_SHARING_MODE_EXCLUSIVE ;

	image_ci.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED ;

	if (!check_vk(dev.vk.CreateImage(dev.vk_device , &image_ci , nullptr , &im.image) , error , "Crear imagen compartida"))
		return false ;

	VkMemoryRequirements req ;

	dev.vk.GetImageMemoryRequirements(dev.vk_device , im.image , &req) ;

	VkPhysicalDeviceMemoryProperties mem_props ;

	dev.vk.GetPhysicalDeviceMemoryProperties(dev.vk_physical , &mem_props) ;

	std :: uint32_t type = UINT32_MAX ;

	for (std :: uint32_t i = 0 ; i < mem_props.memoryTypeCount ; ++i) {

		if ((req.memoryTypeBits & (1u << i)) && (mem_props.memoryTypes[i].propertyFlags & VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT)) {

			type = i ;

			break ;

		}

	}

	if (type == UINT32_MAX) return gpu_fallo(error , "No hay memoria GPU exportable") ;

	auto export_info = vk_struct<VkExportMemoryAllocateInfo>(VK_STRUCTURE_TYPE_EXPORT_MEMORY_ALLOCATE_INFO) ;

	export_info.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT ;

	auto dedicated = vk_struct<VkMemoryDedicatedAllocateInfo>(VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO , &export_info) ;

	dedicated.image = im.image ;

	auto alloc = vk_struct<VkMemoryAllocateInfo>(VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO , &dedicated) ;

	alloc.allocationSize = req.size ;

	alloc.memoryTypeIndex = type ;

	if (!check_vk(dev.vk.AllocateMemory(dev.vk_device , &alloc , nullptr , &im.memory) , error , "Reservar memoria compartida"))
		return false ;

	if (!check_vk(dev.vk.BindImageMemory(dev.vk_device , im.image , im.memory , 0) , error , "Asociar memoria compartida"))
		return false ;

	im.bytes = req.size ;

	auto fd_info = vk_struct<VkMemoryGetFdInfoKHR>(VK_STRUCTURE_TYPE_MEMORY_GET_FD_INFO_KHR) ;

	fd_info.memory = im.memory ;

	fd_info.handleType = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT ;

	if (!check_vk(dev.vk.GetMemoryFdKHR(dev.vk_device , &fd_info , &im.fd_memoria) , error , "Exportar imagen Vulkan"))
		return false ;

	auto sem_export = vk_struct<VkExportSemaphoreCreateInfo>(VK_STRUCTURE_TYPE_EXPORT_SEMAPHORE_CREATE_INFO) ;

	sem_export.handleTypes = VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_FD_BIT ;

	auto sem_ci = vk_struct<VkSemaphoreCreateInfo>(VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO , &sem_export) ;

	if (!check_vk(dev.vk.CreateSemaphore(dev.vk_device , &sem_ci , nullptr , &im.vk_ready) , error , "Crear semáforo Vulkan→EGL")
			|| !check_vk(dev.vk.CreateSemaphore(dev.vk_device , &sem_ci , nullptr , &im.gl_ready) , error , "Crear semáforo EGL→Vulkan"))
		return false ;

	auto sem_fd = vk_struct<VkSemaphoreGetFdInfoKHR>(VK_STRUCTURE_TYPE_SEMAPHORE_GET_FD_INFO_KHR) ;

	sem_fd.handleType = VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_FD_BIT ;

	sem_fd.semaphore = im.vk_ready ;

	if (!check_vk(dev.vk.GetSemaphoreFdKHR(dev.vk_device , &sem_fd , &im.fd_vk) , error , "Exportar semáforo Vulkan"))
		return false ;

	sem_fd.semaphore = im.gl_ready ;

	if (!check_vk(dev.vk.GetSemaphoreFdKHR(dev.vk_device , &sem_fd , &im.fd_gl) , error , "Exportar semáforo EGL"))
		return false ;

	TextureDesc desc ;

	desc.Name = nombre.c_str() ;

	desc.Type = RESOURCE_DIM_TEX_2D ;

	desc.Width = w ;

	desc.Height = h ;

	desc.Format = TEX_FORMAT_RGBA8_UNORM ;

	desc.Usage = USAGE_DEFAULT ;

	desc.BindFlags = BIND_RENDER_TARGET | BIND_SHADER_RESOURCE ;

	desc.ImmediateContextMask = gpu_mascara() ;

	dev.device_vk->CreateTextureFromVulkanImage(im.image , desc , RESOURCE_STATE_UNDEFINED , &im.textura) ;

	if (!im.textura || !im.textura->GetDefaultView(TEXTURE_VIEW_RENDER_TARGET))
		return gpu_fallo(error , "Diligent no pudo adjuntar la imagen compartida") ;

	return true ;

}

// Hilo del carril, con su cola ya ociosa y GL ya sin ella : lo de Vulkan
void destruir_vk(GpuDevice & dev , Imagen & im) {

	im.textura.Release() ;

	im.region_w = im.region_h = 0 ;

	if (im.vk_ready) dev.vk.DestroySemaphore(dev.vk_device , im.vk_ready , nullptr) ;

	if (im.gl_ready) dev.vk.DestroySemaphore(dev.vk_device , im.gl_ready , nullptr) ;

	if (im.image) dev.vk.DestroyImage(dev.vk_device , im.image , nullptr) ;

	if (im.memory) dev.vk.FreeMemory(dev.vk_device , im.memory , nullptr) ;

	im.vk_ready = im.gl_ready = VK_NULL_HANDLE ;

	im.image = VK_NULL_HANDLE ;

	im.memory = VK_NULL_HANDLE ;

	cerrar_fd(im.fd_memoria) ;

	cerrar_fd(im.fd_vk) ;

	cerrar_fd(im.fd_gl) ;

}

// Hilo raster : vacía los errores GL pendientes de este contexto, con tope
// (con el contexto perdido, glGetError podría no vaciarse nunca)
void limpiar_errores_gl() {

	for (int i = 0 ; i < 32 && glGetError() != GL_NO_ERROR ; i++) {}

}

// Hilo raster : la imagen en GL. La textura se adjunta a un FBO y se limpia
// para forzar a EGL a instanciar su vista del almacenamiento importado antes
// de que Vulkan escriba. Cada fd importado pasa a ser de GL; si GL lo
// rechaza, sigue siendo nuestro (lo cierra destruir_vk)
bool importar(Imagen & im , std :: uint32_t w , std :: uint32_t h , GError ** error) {

	// los errores de antes no son de esta importación

	limpiar_errores_gl() ;

	glCreateMemoryObjectsEXT(1 , &im.gl_memory) ;

	GLint yes = GL_TRUE ;

	glMemoryObjectParameterivEXT(im.gl_memory , GL_DEDICATED_MEMORY_OBJECT_EXT , &yes) ;

	glImportMemoryFdEXT(im.gl_memory , im.bytes , GL_HANDLE_TYPE_OPAQUE_FD_EXT , im.fd_memoria) ;

	if (glGetError() != GL_NO_ERROR) return gpu_fallo(error , "EGL no pudo importar la memoria Vulkan") ;

	im.fd_memoria = -1 ;

	glGenTextures(1 , &im.gl_texture) ;

	GLint old_texture = 0 , old_draw_fbo = 0 , old_read_fbo = 0 ;

	GLfloat old_clear[4] ;

	GLboolean old_mask[4] ;

	GLboolean old_scissor = glIsEnabled(GL_SCISSOR_TEST) ;

	glGetIntegerv(GL_TEXTURE_BINDING_2D , &old_texture) ;

	glGetIntegerv(GL_DRAW_FRAMEBUFFER_BINDING , &old_draw_fbo) ;

	glGetIntegerv(GL_READ_FRAMEBUFFER_BINDING , &old_read_fbo) ;

	glGetFloatv(GL_COLOR_CLEAR_VALUE , old_clear) ;

	glGetBooleanv(GL_COLOR_WRITEMASK , old_mask) ;

	glBindTexture(GL_TEXTURE_2D , im.gl_texture) ;

	glTexStorageMem2DEXT(GL_TEXTURE_2D , 1 , GL_RGBA8 , w , h , im.gl_memory , 0) ;

	glTexParameteri(GL_TEXTURE_2D , GL_TEXTURE_MIN_FILTER , GL_NEAREST) ;

	glTexParameteri(GL_TEXTURE_2D , GL_TEXTURE_MAG_FILTER , GL_NEAREST) ;

	glTexParameteri(GL_TEXTURE_2D , GL_TEXTURE_WRAP_S , GL_CLAMP_TO_EDGE) ;

	glTexParameteri(GL_TEXTURE_2D , GL_TEXTURE_WRAP_T , GL_CLAMP_TO_EDGE) ;

	GLuint framebuffer = 0 ;

	glGenFramebuffers(1 , &framebuffer) ;

	glBindFramebuffer(GL_FRAMEBUFFER , framebuffer) ;

	glFramebufferTexture2D(GL_FRAMEBUFFER , GL_COLOR_ATTACHMENT0 , GL_TEXTURE_2D , im.gl_texture , 0) ;

	GLenum status = glCheckFramebufferStatus(GL_FRAMEBUFFER) ;

	glDisable(GL_SCISSOR_TEST) ;

	glColorMask(GL_TRUE , GL_TRUE , GL_TRUE , GL_TRUE) ;

	glClearColor(0 , 0 , 0 , 0) ;

	glClear(GL_COLOR_BUFFER_BIT) ;

	glBindFramebuffer(GL_READ_FRAMEBUFFER , old_read_fbo) ;

	glBindFramebuffer(GL_DRAW_FRAMEBUFFER , old_draw_fbo) ;

	glDeleteFramebuffers(1 , &framebuffer) ;

	glBindTexture(GL_TEXTURE_2D , old_texture) ;

	glClearColor(old_clear[0] , old_clear[1] , old_clear[2] , old_clear[3]) ;

	glColorMask(old_mask[0] , old_mask[1] , old_mask[2] , old_mask[3]) ;

	if (old_scissor) glEnable(GL_SCISSOR_TEST) ;

	if (status != GL_FRAMEBUFFER_COMPLETE || glGetError() != GL_NO_ERROR)
		return gpu_fallo(error , "EGL no pudo importar la imagen Vulkan") ;

	glGenSemaphoresEXT(1 , &im.gl_vk_ready) ;

	glGenSemaphoresEXT(1 , &im.gl_gl_ready) ;

	glImportSemaphoreFdEXT(im.gl_vk_ready , GL_HANDLE_TYPE_OPAQUE_FD_EXT , im.fd_vk) ;

	if (glGetError() != GL_NO_ERROR) return gpu_fallo(error , "EGL no pudo importar los semáforos Vulkan") ;

	im.fd_vk = -1 ;

	glImportSemaphoreFdEXT(im.gl_gl_ready , GL_HANDLE_TYPE_OPAQUE_FD_EXT , im.fd_gl) ;

	if (glGetError() != GL_NO_ERROR) return gpu_fallo(error , "EGL no pudo importar los semáforos Vulkan") ;

	im.fd_gl = -1 ;

	return true ;

}

// Hilo raster : lo de GL
void soltar_gl(Imagen & im) {

	if (im.gl_vk_ready) glDeleteSemaphoresEXT(1 , &im.gl_vk_ready) ;

	if (im.gl_gl_ready) glDeleteSemaphoresEXT(1 , &im.gl_gl_ready) ;

	if (im.gl_texture) glDeleteTextures(1 , &im.gl_texture) ;

	if (im.gl_memory) glDeleteMemoryObjectsEXT(1 , &im.gl_memory) ;

	im.gl_vk_ready = im.gl_gl_ready = im.gl_texture = im.gl_memory = 0 ;

}

// Hilo raster : lee la textura por GL y la escribe como PPM tal como se ve en
// pantalla; con `ancho` y `alto`, solo su esquina de arriba a la izquierda
void volcar_textura(GLuint textura , std :: uint32_t tw , std :: uint32_t th , const std :: string & ruta , std :: uint32_t ancho , std :: uint32_t alto) {

	const std :: uint32_t w = ancho && ancho < tw ? ancho : tw ;

	const std :: uint32_t h = alto && alto < th ? alto : th ;

	GLint old_read = 0 , old_draw = 0 ;

	glGetIntegerv(GL_READ_FRAMEBUFFER_BINDING , &old_read) ;

	glGetIntegerv(GL_DRAW_FRAMEBUFFER_BINDING , &old_draw) ;

	GLuint fbo = 0 ;

	glGenFramebuffers(1 , &fbo) ;

	glBindFramebuffer(GL_FRAMEBUFFER , fbo) ;

	glFramebufferTexture2D(GL_FRAMEBUFFER , GL_COLOR_ATTACHMENT0 , GL_TEXTURE_2D , textura , 0) ;

	std :: vector<std :: uint8_t> px(static_cast<std :: size_t>(w) * h * 4) ;

	const bool leido = glCheckFramebufferStatus(GL_FRAMEBUFFER) == GL_FRAMEBUFFER_COMPLETE ;

	if (leido) glReadPixels(0 , 0 , w , h , GL_RGBA , GL_UNSIGNED_BYTE , px.data()) ;

	glBindFramebuffer(GL_READ_FRAMEBUFFER , old_read) ;

	glBindFramebuffer(GL_DRAW_FRAMEBUFFER , old_draw) ;

	glDeleteFramebuffers(1 , &fbo) ;

	limpiar_errores_gl() ;

	if (!leido) return ;

	const std :: string tmp = ruta + ".tmp" ;

	FILE * f = std :: fopen(tmp.c_str() , "wb") ;

	if (!f) return ;

	std :: fprintf(f , "P6\n%u %u\n255\n" , w , h) ;

	// Diligent dibuja con el viewport invertido en Vulkan, así que la primera
	// fila en memoria (la fila 0 de GL) ya es la de arriba en pantalla: se
	// escribe en el mismo orden en que se lee.
	std :: vector<std :: uint8_t> fila(static_cast<std :: size_t>(w) * 3) ;

	for (std :: uint32_t y = 0 ; y < h ; ++y) {

		const std :: uint8_t * src = px.data() + static_cast<std :: size_t>(y) * w * 4 ;

		for (std :: uint32_t x = 0 ; x < w ; ++x) {

			fila[x * 3] = src[x * 4] ;

			fila[x * 3 + 1] = src[x * 4 + 1] ;

			fila[x * 3 + 2] = src[x * 4 + 2] ;

		}

		std :: fwrite(fila.data() , 1 , fila.size() , f) ;

	}

	std :: fclose(f) ;

	std :: rename(tmp.c_str() , ruta.c_str()) ;

}

}  // namespace

struct GpuSurface :: Impl {

	std :: string nombre ;

	Carril & carril ;

	std :: size_t trabajo = 0 ;

	// del hilo de plataforma, al montar el renderizador

	std :: function<bool(GError **)> pintar ;

	std :: mutex aviso_mutex ;

	std :: function<void()> aviso ;

	// la fase, el tamaño, los volcados y el fallo, entre el carril y el raster

	std :: mutex mutex ;

	Fase fase = Fase :: sin_imagenes ;

	bool recien = false ;

	std :: uint32_t width = 0 , height = 0 ;

	Imagen imagen[total_imagenes] ;

	Relevo relevo{ total_imagenes } ;

	std :: string fallo ;

	// el carril : la que pinta ahora, o -1

	int actual = -1 ;

	// el carril se quedó sin imagen libre : que el raster lo vuelva a pedir
	// al coger la lista

	std :: atomic<bool> sin_hueco{ false } ;

	// el raster

	bool mostro = false ;

	GLuint provisional = 0 ;

	// la importación en GL falló : no se reintenta en cada fotograma (dejaría
	// objetos GL por el camino); el fallo se queda

	std :: string fallo_gl ;

	Impl(const char * n , Carril & c) : nombre(n) , carril(c) {}

	void avisar() {

		std :: function<void()> a ;

		{

			std :: lock_guard<std :: mutex> lock(aviso_mutex) ;

			a = aviso ;

		}

		if (a) a() ;

	}

	// Hilo del carril, cada vez que se pide
	void correr() {

		if (!pintar) return ;

		GError * error = nullptr ;

		if (pintar(&error)) return ;

		// tomada a medias : vuelve libre

		if (actual >= 0) {

			relevo.devolver(actual) ;

			actual = -1 ;

		}

		{

			std :: lock_guard<std :: mutex> lock(mutex) ;

			fallo = error && error->message ? error->message : "el carril no pudo pintar" ;

		}

		g_clear_error(&error) ;

		// que el raster lo cuente

		avisar() ;

	}

	GpuDevice & dev() const { return dispositivo() ; }

	IDeviceContext * ctx() const { return gpu_context(carril.indice()) ; }

	// Hilo del carril : un envío propio a la cola de su contexto (los relevos
	// de semáforos con GL), en orden con lo que envía Diligent
	void enviar(const VkSubmitInfo & info) const {

		ICommandQueue * cola = ctx()->LockCommandQueue() ;

		static_cast<ICommandQueueVk *>(cola)->Submit(info) ;

		ctx()->UnlockCommandQueue() ;

	}

} ;

GpuSurface :: GpuSurface(const char * nombre , Carril & carril) : impl_(new Impl(nombre , carril)) {

	impl_->trabajo = carril.registrar([this] { impl_->correr() ; }) ;

}

// Sin llamadas GL ni Vulkan: el destructor puede ejecutarse fuera de los dos
// hilos; los handles externos los libera el proceso al salir. El carril no
// debe volver a correr su trabajo (los renderizadores viven lo que la app).
GpuSurface :: ~GpuSurface() { delete impl_ ; }

void GpuSurface :: al_pintar(std :: function<bool(GError **)> pintar) { impl_->pintar = std :: move(pintar) ; }

void GpuSurface :: avisar_con(std :: function<void()> aviso) {

	std :: lock_guard<std :: mutex> lock(impl_->aviso_mutex) ;

	impl_->aviso = std :: move(aviso) ;

}

void GpuSurface :: pedir() { impl_->carril.pedir(impl_->trabajo) ; }

IRenderDevice * GpuSurface :: device() const { return gpu_device() ; }

IDeviceContext * GpuSurface :: context() const { return impl_->ctx() ; }

std :: uint32_t GpuSurface :: width() const { return impl_->width ; }

std :: uint32_t GpuSurface :: height() const { return impl_->height ; }

void * GpuSurface :: ranura_estado(const void * etiqueta , std :: shared_ptr<void> (* crear)()) {

	auto & d = dispositivo() ;

	std :: lock_guard<std :: mutex> lock(d.estados_mutex) ;

	std :: shared_ptr<void> & ranura = d.estados[etiqueta] ;

	if (!ranura) ranura = crear() ;

	return ranura.get() ;

}

bool GpuSurface :: acomodar(std :: uint32_t w , std :: uint32_t h , Acomodo * acomodo , GError ** error) {

	auto & s = * impl_ ;

	* acomodo = Acomodo :: esperando ;

	std :: unique_lock<std :: mutex> lock(s.mutex) ;

	if (s.fase == Fase :: lista) {

		if (s.width == w && s.height == h) {

			* acomodo = s.recien ? Acomodo :: nueva : Acomodo :: lista ;

			s.recien = false ;

			return true ;

		}

		// otro tamaño (la vista pasó del lienzo) : que el raster las suelte

		s.fase = Fase :: por_retirar ;

		lock.unlock() ;

		s.avisar() ;

		return true ;

	}

	if (s.fase == Fase :: por_importar || s.fase == Fase :: por_retirar) return true ;

	if (s.fase == Fase :: retirada) {

		// GL ya las soltó; en la GPU, que acabe lo de esta cola

		lock.unlock() ;

		s.ctx()->WaitForIdle() ;

		for (Imagen & im : s.imagen) destruir_vk(s.dev() , im) ;

		lock.lock() ;

		s.fase = Fase :: sin_imagenes ;

	}

	lock.unlock() ;

	for (Imagen & im : s.imagen) {

		if (!crear_vk(s.dev() , s.nombre , im , w , h , error)) {

			for (Imagen & otra : s.imagen) destruir_vk(s.dev() , otra) ;

			return false ;

		}

	}

	lock.lock() ;

	s.width = w ;

	s.height = h ;

	s.fase = Fase :: por_importar ;

	lock.unlock() ;

	// que el raster las importe en GL antes de que Vulkan escriba

	s.avisar() ;

	return true ;

}

bool GpuSurface :: comenzar(bool * hay , GError **) {

	auto & s = * impl_ ;

	const int i = s.relevo.tomar_libre() ;

	if (i < 0) {

		s.sin_hueco = true ;

		* hay = false ;

		return true ;

	}

	s.actual = i ;

	* hay = true ;

	// GL la mostró y la soltó : Vulkan espera su semáforo antes de escribir

	if (s.relevo.consumir_espera(i)) {

		VkPipelineStageFlags etapa = VK_PIPELINE_STAGE_ALL_COMMANDS_BIT ;

		auto espera = vk_struct<VkSubmitInfo>(VK_STRUCTURE_TYPE_SUBMIT_INFO) ;

		espera.waitSemaphoreCount = 1 ;

		espera.pWaitSemaphores = &s.imagen[i].gl_ready ;

		espera.pWaitDstStageMask = &etapa ;

		s.enviar(espera) ;

	}

	return true ;

}

void GpuSurface :: fijar_objetivo(const float fondo[4]) {

	auto & s = * impl_ ;

	if (s.actual < 0) {

		g_critical("N-LTX GPU: fijar_objetivo sin imagen tomada (falta comenzar)") ;

		return ;

	}

	IDeviceContext * ctx = s.ctx() ;

	ITextureView * rtv = s.imagen[s.actual].textura->GetDefaultView(TEXTURE_VIEW_RENDER_TARGET) ;

	ctx->SetRenderTargets(1 , &rtv , nullptr , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

	ctx->ClearRenderTarget(rtv , fondo , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

	Viewport vista ;

	vista.TopLeftX = 0 ;

	vista.TopLeftY = 0 ;

	vista.Width = static_cast<float>(s.width) ;

	vista.Height = static_cast<float>(s.height) ;

	vista.MinDepth = 0 ;

	vista.MaxDepth = 1 ;

	ctx->SetViewports(1 , &vista , s.width , s.height) ;

}

void GpuSurface :: fijar_region(std :: uint32_t ancho , std :: uint32_t alto , const float fondo[4]) {

	auto & s = * impl_ ;

	if (s.actual < 0) {

		g_critical("N-LTX GPU: fijar_region sin imagen tomada (falta comenzar)") ;

		return ;

	}

	IDeviceContext * ctx = s.ctx() ;

	Imagen & im = s.imagen[s.actual] ;

	ITextureView * rtv = im.textura->GetDefaultView(TEXTURE_VIEW_RENDER_TARGET) ;

	ctx->SetRenderTargets(1 , &rtv , nullptr , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

	// otra región que la última vez : la imagen entera, al fondo. Lo que
	// quedaba fuera de la nueva era de otro tamaño y asomaba en el borde, o
	// al agrandar la vista antes de su fotograma. Con la misma región, no
	// cuesta nada

	if (im.region_w != ancho || im.region_h != alto) {

		ctx->ClearRenderTarget(rtv , fondo , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		im.region_w = ancho ;

		im.region_h = alto ;

	}

	Viewport vista ;

	vista.TopLeftX = 0 ;

	vista.TopLeftY = 0 ;

	vista.Width = static_cast<float>(std :: min(ancho , s.width)) ;

	vista.Height = static_cast<float>(std :: min(alto , s.height)) ;

	vista.MinDepth = 0 ;

	vista.MaxDepth = 1 ;

	ctx->SetViewports(1 , &vista , s.width , s.height) ;

}

void GpuSurface :: volcar(const char * ruta , std :: uint32_t ancho , std :: uint32_t alto) {

	auto & s = * impl_ ;

	if (!ruta || s.actual < 0) return ;

	std :: lock_guard<std :: mutex> lock(s.mutex) ;

	Imagen & im = s.imagen[s.actual] ;

	im.volcado = ruta ;

	im.volcado_w = ancho ;

	im.volcado_h = alto ;

}

bool GpuSurface :: terminar(GError ** error) {

	auto & s = * impl_ ;

	if (s.actual < 0) return gpu_fallo(error , "terminar sin imagen tomada (falta comenzar)") ;

	auto & d = s.dev() ;

	IDeviceContext * ctx = s.ctx() ;

	Imagen & im = s.imagen[s.actual] ;

	d.contexts_vk[s.carril.indice()]->TransitionImageLayout(im.textura , VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL) ;

	ctx->Flush() ;

	auto aviso = vk_struct<VkSubmitInfo>(VK_STRUCTURE_TYPE_SUBMIT_INFO) ;

	aviso.signalSemaphoreCount = 1 ;

	aviso.pSignalSemaphores = &im.vk_ready ;

	s.enviar(aviso) ;

	// Sin swapchain de Diligent: cerrar el frame y reciclar el upload heap.
	ctx->FinishFrame() ;

	{

		std :: lock_guard<std :: mutex> lock(d.liberar) ;

		d.device->ReleaseStaleResources() ;

	}

	// Terminada en la GPU antes de publicarla : la espera es del carril, el
	// raster coge solo lo acabado

	ctx->WaitForIdle() ;

	s.relevo.publicar(s.actual) ;

	s.actual = -1 ;

	s.avisar() ;

	return true ;

}

bool GpuSurface :: mostrar(

	std :: uint32_t * target ,

	std :: uint32_t * name ,

	std :: uint32_t * width ,

	std :: uint32_t * height ,

	bool * nueva ,

	GError ** error

){

	auto & s = * impl_ ;

	auto & d = dispositivo() ;

	* nueva = false ;

	if (!leer_gl(error)) return false ;

	{

		std :: lock_guard<std :: mutex> lock(d.mutex) ;

		if (!d.fallo.empty()) return gpu_fallo(error , d.fallo.c_str()) ;

	}

	bool pedir_luego = false ;

	std :: string volcado ;

	std :: uint32_t volcado_w = 0 , volcado_h = 0 ;

	GLuint textura = 0 ;

	std :: uint32_t tw = 1 , th = 1 ;

	{

		std :: lock_guard<std :: mutex> lock(s.mutex) ;

		if (!s.fallo_gl.empty()) return gpu_fallo(error , s.fallo_gl.c_str()) ;

		// imágenes nuevas del carril : en GL, antes de que Vulkan escriba

		if (s.fase == Fase :: por_importar) {

			for (Imagen & im : s.imagen) {

				if (!importar(im , s.width , s.height , error)) {

					// lo importado se suelta, y no se vuelve a intentar

					for (Imagen & otra : s.imagen) soltar_gl(otra) ;

					s.fallo_gl = error && * error ? (* error)->message : "EGL no pudo importar la imagen Vulkan" ;

					return false ;

				}

			}

			s.relevo.reiniciar() ;

			s.fase = Fase :: lista ;

			s.recien = true ;

			pedir_luego = true ;

		}

		// el carril las retira (otro tamaño) : GL las suelta

		if (s.fase == Fase :: por_retirar) {

			for (Imagen & im : s.imagen) soltar_gl(im) ;

			glFinish() ;

			s.relevo.reiniciar() ;

			s.fase = Fase :: retirada ;

			pedir_luego = true ;

		}

		if (s.fase == Fase :: lista) {

			// La que deja de verse se suelta antes de cambiarla en el relevo :
			// cuando el carril la tome, el aviso de GL ya está enviado

			const int vieja = s.relevo.mostrada() ;

			GLenum layout = GL_LAYOUT_SHADER_READ_ONLY_EXT ;

			if (s.relevo.hay_lista() && vieja >= 0) {

				glSignalSemaphoreEXT(s.imagen[vieja].gl_gl_ready , 0 , nullptr , 1 , &s.imagen[vieja].gl_texture , &layout) ;

				glFlush() ;

			}

			const Relevo :: Cambio c = s.relevo.tomar_lista() ;

			if (c.mostrar >= 0) {

				Imagen & im = s.imagen[c.mostrar] ;

				// ya terminada en la GPU : esperarla no frena a Flutter

				glWaitSemaphoreEXT(im.gl_vk_ready , 0 , nullptr , 1 , &im.gl_texture , &layout) ;

				* nueva = true ;

				s.mostro = true ;

				if (!im.volcado.empty()) {

					volcado = std :: move(im.volcado) ;

					im.volcado.clear() ;

					volcado_w = im.volcado_w ;

					volcado_h = im.volcado_h ;

				}

				if (s.sin_hueco.exchange(false)) pedir_luego = true ;

			}

			const int m = s.relevo.mostrada() ;

			if (m >= 0) {

				textura = s.imagen[m].gl_texture ;

				tw = s.width ;

				th = s.height ;

			}

		}

		// un fallo del carril : sin nada que mostrar, a Dart; si ya hubo, se
		// conserva el último fotograma

		if (!s.fallo.empty()) {

			if (!s.mostro) return gpu_fallo(error , s.fallo.c_str()) ;

			g_warning("N-LTX GPU: se conserva el último fotograma de %s: %s" , s.nombre.c_str() , s.fallo.c_str()) ;

			s.fallo.clear() ;

		}

	}

	if (!textura) {

		// la provisional, transparente, mientras no hay ninguna

		if (!s.provisional) {

			const std :: uint8_t nada[4] = { 0 , 0 , 0 , 0 } ;

			GLint old_texture = 0 ;

			glGetIntegerv(GL_TEXTURE_BINDING_2D , &old_texture) ;

			glGenTextures(1 , &s.provisional) ;

			glBindTexture(GL_TEXTURE_2D , s.provisional) ;

			glTexImage2D(GL_TEXTURE_2D , 0 , GL_RGBA8 , 1 , 1 , 0 , GL_RGBA , GL_UNSIGNED_BYTE , nada) ;

			glTexParameteri(GL_TEXTURE_2D , GL_TEXTURE_MIN_FILTER , GL_NEAREST) ;

			glTexParameteri(GL_TEXTURE_2D , GL_TEXTURE_MAG_FILTER , GL_NEAREST) ;

			glBindTexture(GL_TEXTURE_2D , old_texture) ;

		}

		textura = s.provisional ;

	}

	if (!volcado.empty()) volcar_textura(textura , tw , th , volcado , volcado_w , volcado_h) ;

	if (pedir_luego) pedir() ;

	* target = GL_TEXTURE_2D ;

	* name = textura ;

	* width = tw ;

	* height = th ;

	return true ;

}
