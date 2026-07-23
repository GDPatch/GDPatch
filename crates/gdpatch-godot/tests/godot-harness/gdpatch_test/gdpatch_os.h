#pragma once

#include <cstdint>

#include "core/object/class_db.h"
#include "core/object/ref_counted.h"

class GDPatchOS : public RefCounted {
    GDCLASS(GDPatchOS, RefCounted);

    protected:
        static void _bind_methods();
        static PackedByteArray get_stdin_buffer(int64_t p_buffer_size);
};
