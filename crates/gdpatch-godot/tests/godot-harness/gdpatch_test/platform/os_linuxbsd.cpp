#include "gdpatch_os.h"
#include "core/object/class_db.h"

#include <cstdio>

PackedByteArray GDPatchOS::get_stdin_buffer(int64_t p_buffer_size) {
    Vector<uint8_t> data;
    data.resize(p_buffer_size);
    size_t sz = fread((void *)data.ptrw(), 1, data.size(), stdin);
    if (sz > 0) {
        data.resize(sz);
        return data;
    }

    return {};
}