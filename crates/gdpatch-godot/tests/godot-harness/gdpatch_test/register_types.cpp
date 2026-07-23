#include "register_types.h"

#include "gdpatch_json.h"
#include "core/object/class_db.h"
#include "tokenizer.h"
#include "gdpatch_os.h"

void initialize_gdpatch_test_module(ModuleInitializationLevel p_level) {
    if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
        return;
    }

    ClassDB::register_class<GDPatchOS>();
    ClassDB::register_class<BoundToken>();
    ClassDB::register_class<BoundGDTokenizerText>();
    ClassDB::register_class<GDPatchJson>();

#ifdef GDPATCH_HAS_TOKENIZER_BUFFER
    ClassDB::register_class<BoundGDTokenizerBuffer>();
#endif
}

void uninitialize_gdpatch_test_module(ModuleInitializationLevel p_level) {
    if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
        return;
    }
}
