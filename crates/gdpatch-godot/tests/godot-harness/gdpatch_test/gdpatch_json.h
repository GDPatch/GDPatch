#pragma once

#include "core/object/class_db.h"
#include "thirdparty/json.hpp"
#include "core/object/ref_counted.h"

class GDPatchJson : public RefCounted {
    GDCLASS(GDPatchJson, RefCounted);

    protected:
        static void _bind_methods();
        static Variant stringify(Variant var);
};