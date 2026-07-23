#pragma once
#include "core/object/ref_counted.h"
#include "core/object/class_db.h"
#include "modules/gdscript/gdscript_tokenizer.h"

#ifdef GDPATCH_HAS_TOKENIZER_BUFFER
#include "modules/gdscript/gdscript_tokenizer_buffer.h"
#else
typedef GDScriptTokenizer GDScriptTokenizerText ;
#endif

class BoundToken : public RefCounted {
    GDCLASS(BoundToken, RefCounted);
    GDScriptTokenizer::Token inner;

protected:
    static void _bind_methods();
    int get_type() const;
    String get_name() const;
    Variant get_literal() const;

public:
    friend class BoundGDTokenizerText;
    friend class BoundGDTokenizerBuffer;
    BoundToken() = default;
};

class BoundGDTokenizerText : public RefCounted {
    GDCLASS(BoundGDTokenizerText, RefCounted);
    GDScriptTokenizerText inner;

protected:
    static void _bind_methods();
    void set_source_code(const String& source);
    Ref<BoundToken> scan();

public:
    BoundGDTokenizerText() = default;
};

#ifdef GDPATCH_HAS_TOKENIZER_BUFFER
class BoundGDTokenizerBuffer : public RefCounted {
    GDCLASS(BoundGDTokenizerBuffer, RefCounted);
    GDScriptTokenizerBuffer inner;

protected:
    static PackedByteArray parse_code_string(const String &source);
    static void _bind_methods();
    Error set_code_buffer(const Vector<uint8_t> &buffer);
    Ref<BoundToken> scan();

public:
    BoundGDTokenizerBuffer() = default;
};
#endif
