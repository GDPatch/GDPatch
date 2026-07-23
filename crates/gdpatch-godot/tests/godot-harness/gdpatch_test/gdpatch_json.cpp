#include "gdpatch_json.h"
#include "thirdparty/json.hpp"
#include "gdpatch_version_compat.h"
#include "core/object/class_db.h"

using namespace nlohmann;

static json make_json(Variant var)
{
    Variant::Type type = var.get_type();
    switch(type)
    {
        case Variant::Type::ARRAY:
        {
            Array arr = static_cast<Array>(var);
            std::vector<json> elements;
            for (int i = 0; i < arr.size(); i++)
            {
                elements.push_back(make_json(arr.get(i)));
            }

            return json(elements);
        }

        case Variant::Type::DICTIONARY:
        {
            std::map<std::string, json> map;
            Dictionary dictionary = static_cast<Dictionary>(var);

            #if GODOT_VERSION_HEX >= 0x040500
            LocalVector<Variant> keys = dictionary.get_key_list();
            #else
            List<Variant> keys;
            dictionary.get_key_list(&keys);
            #endif

            for (const Variant& key : keys)
            {
                std::string chars = static_cast<String>(key).utf8().get_data();
                json value = make_json(dictionary[key]);
                map[chars] = json(value);
            }

            return json(map);
        }

        case Variant::Type::BOOL:
        {
            return json(static_cast<bool>(var));
        }
        case Variant::Type::INT:
        {
            return json(static_cast<int>(var));
        }
        case Variant::Type::FLOAT:
        {
            return json(static_cast<double>(var));
        }
        case Variant::Type::STRING:
        case Variant::Type::STRING_NAME:
        case Variant::Type::NODE_PATH:
        {
            CharString cstr = static_cast<String>(var).utf8();
            return json(cstr.get_data());
        }

        case Variant::Type::NIL:
        {
            return json(nullptr);
        }

        default: return json(nullptr);
    }
}

Variant GDPatchJson::stringify(Variant var)
{
    std::string str = make_json(var).dump();
    return Variant(String::utf8(str.c_str(), str.length()));
}

void GDPatchJson::_bind_methods()
{
    ClassDB::bind_static_method("GDPatchJson", D_METHOD("stringify", "var"), &GDPatchJson::stringify);
}
