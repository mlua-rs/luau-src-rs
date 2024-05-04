#include "Luau/Common.h"

#include <string.h>

extern "C" int luau_setfflag(const char* name, int value)
{
    for (Luau::FValue<bool>* flag = Luau::FValue<bool>::list; flag; flag = flag->next)
    {
        if (strcmp(flag->name, name) == 0)
        {
            flag->value = value;
            return 1;
        }
    }
    return 0;
}
