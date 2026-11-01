# include "banco_gpu.h"

# include <flutter_linux/flutter_linux.h>

# include <algorithm>

# include <array>

# include <chrono>

# include <cstdio>

# include <ctime>

# include "Query.h"

# include "RefCntAutoPtr.hpp"

using namespace Diligent ;

namespace {

constexpr std :: size_t fotogramas_por_informe = 300 ;

constexpr const char * nombres_etapa[] = { "preparar" , "subir" , "dibujar" } ;

// CPU que lleva gastada este hilo, en ms

double cpu_hilo() {

	timespec t{} ;

	clock_gettime(CLOCK_THREAD_CPUTIME_ID , &t) ;

	return static_cast<double>(t.tv_sec) * 1e3 + static_cast<double>(t.tv_nsec) * 1e-6 ;

}

struct Muestras {

	std :: vector<double> v ;

	void poner(double x) { v.push_back(x) ; }

	// ordena y da el cuantil q (0,5 : la mediana); sin muestras, 0

	double cuantil(double q) {

		if (v.empty()) { return 0 ; }

		std :: sort(v.begin() , v.end()) ;

		return v[std :: min(v.size() - 1 , static_cast<std :: size_t>(q * static_cast<double>(v.size())))] ;

	}

} ;

struct Medida {

	double cpu0 = 0 ;

	Muestras cpu , gpu ;

	RefCntAutoPtr<IQuery> consulta ;

	bool abierta = false ;

	bool sin_consultas = false ;

} ;

}  // namespace

struct BancoGpu :: Impl {

	std :: string prefijo ;

	std :: vector<std :: string> nombres ;

	std :: vector<std :: array<Medida , 3>> medidas ;

	double cpu0 = 0 ;

	std :: chrono :: steady_clock :: time_point t0 ;

	Muestras fotograma , cpu ;

	std :: size_t n = 0 ;

} ;

BancoGpu :: BancoGpu(std :: string prefijo , std :: vector<std :: string> pasadas) : impl_(new Impl()) {

	impl_->prefijo = std :: move(prefijo) ;


	impl_->medidas.resize(pasadas.size()) ;

	impl_->nombres = std :: move(pasadas) ;

}

BancoGpu :: ~BancoGpu() { delete impl_ ; }

void BancoGpu :: empezar() { impl_->cpu0 = cpu_hilo() ; }

void BancoGpu :: comenzar() { impl_->t0 = std :: chrono :: steady_clock :: now() ; }

void BancoGpu :: abrir(std :: size_t pasada , Etapa etapa , IRenderDevice * device , IDeviceContext * ctx) {

	Medida & m = impl_->medidas[pasada][static_cast<std :: size_t>(etapa)] ;

	if (ctx && device && !m.consulta && !m.sin_consultas) {

		QueryDesc d ;

		d.Name = "N-LTX banco" ;

		d.Type = QUERY_TYPE_DURATION ;

		device->CreateQuery(d , &m.consulta) ;

		// sin DurationQueries en el dispositivo : solo CPU

		m.sin_consultas = !m.consulta ;

	}

	if (ctx && m.consulta) {

		ctx->BeginQuery(m.consulta) ;

		m.abierta = true ;

	}

	m.cpu0 = cpu_hilo() ;

}

void BancoGpu :: cerrar(std :: size_t pasada , Etapa etapa , IDeviceContext * ctx) {

	Medida & m = impl_->medidas[pasada][static_cast<std :: size_t>(etapa)] ;

	m.cpu.poner(cpu_hilo() - m.cpu0) ;

	if (ctx && m.abierta) { ctx->EndQuery(m.consulta) ; }

}

void BancoGpu :: terminar(IDeviceContext * ctx , std :: uint32_t ancho , std :: uint32_t alto) {

	auto & s = * impl_ ;

	s.cpu.poner(cpu_hilo() - s.cpu0) ;

	ctx->WaitForIdle() ;

	const std :: chrono :: duration<double , std :: milli> ms = std :: chrono :: steady_clock :: now() - s.t0 ;

	s.fotograma.poner(ms.count()) ;

	for (auto & etapas : s.medidas) {

		for (Medida & m : etapas) {

			if (!m.abierta) { continue ; }

			m.abierta = false ;

			QueryDataDuration d ;

			if (m.consulta->GetData(&d , sizeof(d)) && d.Frequency) {

				m.gpu.poner(static_cast<double>(d.Duration) * 1e3 / static_cast<double>(d.Frequency)) ;

			}

		}

	}

	if (++s.n < fotogramas_por_informe) { return ; }

	g_message(

		"%s BANCO GPU: %zu fotogramas %ux%u, min %.3f ms, mediana %.3f ms, p95 %.3f ms; "

		"CPU del carril mediana %.3f ms, p95 %.3f ms" ,

		s.prefijo.c_str() ,

		s.n ,

		ancho ,

		alto ,

		s.fotograma.cuantil(0) ,

		s.fotograma.cuantil(0.5) ,

		s.fotograma.cuantil(0.95) ,

		s.cpu.cuantil(0.5) ,

		s.cpu.cuantil(0.95)

	) ;

	for (std :: size_t i = 0 ; i < s.nombres.size() ; i++) {

		std :: string linea ;

		for (std :: size_t e = 0 ; e < 3 ; e++) {

			Medida & m = s.medidas[i][e] ;

			char tramo[160] ;

			std :: snprintf(

				tramo ,

				sizeof(tramo) ,

				" %s CPU %.3f (p95 %.3f)" ,

				nombres_etapa[e] ,

				m.cpu.cuantil(0.5) ,

				m.cpu.cuantil(0.95)

			) ;

			linea += tramo ;

			if (!m.gpu.v.empty()) {

				std :: snprintf(tramo , sizeof(tramo) , " GPU %.3f (p95 %.3f, mín %.3f)" , m.gpu.cuantil(0.5) , m.gpu.cuantil(0.95) , m.gpu.cuantil(0.0)) ;

				linea += tramo ;

			}

			linea += " ;" ;

			m.cpu.v.clear() ;

			m.gpu.v.clear() ;

		}

		g_message("%s BANCO PASADA %s (ms):%s" , s.prefijo.c_str() , s.nombres[i].c_str() , linea.c_str()) ;

	}

	s.fotograma.v.clear() ;

	s.cpu.v.clear() ;

	s.n = 0 ;

}

Tramo :: Tramo(BancoGpu * banco , std :: size_t pasada , Etapa etapa , IRenderDevice * device , IDeviceContext * ctx)

	: banco_(banco) , pasada_(pasada) , etapa_(etapa) , ctx_(ctx) {

	if (banco_) { banco_->abrir(pasada_ , etapa_ , device , ctx_) ; }

}

Tramo :: ~Tramo() {

	if (banco_) { banco_->cerrar(pasada_ , etapa_ , ctx_) ; }

}
