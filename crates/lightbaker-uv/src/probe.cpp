// The sizes of xatlas' C API structs as this compiler lays them out, for the Rust FFI's layout test.
#include "xatlas_c.h"

extern "C" size_t lbuv_sizeof(int which) {
    switch (which) {
    case 0: return sizeof(xatlasMeshDecl);
    case 1: return sizeof(xatlasChartOptions);
    case 2: return sizeof(xatlasPackOptions);
    case 3: return sizeof(xatlasAtlas);
    case 4: return sizeof(xatlasMesh);
    case 5: return sizeof(xatlasVertex);
    case 6: return sizeof(xatlasChart);
    }
    return 0;
}
