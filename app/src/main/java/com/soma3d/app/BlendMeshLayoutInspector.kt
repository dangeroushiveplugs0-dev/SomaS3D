package com.soma3d.app

/**
 * Classifies mesh storage using the file's own SDNA declarations.
 *
 * This is diagnostic/capability detection only: recognizing CustomData does not mean
 * its layer enums or payloads have been decoded, so it never attempts unsafe guesses.
 */
object BlendMeshLayoutInspector {
    enum class Kind {
        LEGACY_DIRECT_ARRAYS,
        CUSTOM_DATA_LAYOUT,
        INCOMPLETE_OR_UNKNOWN
    }

    data class Report(
        val kind: Kind,
        val meshFields: Set<String>,
        val hasCustomDataTypes: Boolean,
        val explanation: String
    )

    fun inspect(schema: BlendBlockReader.Schema): Report {
        val mesh = schema.structs.firstOrNull { it.typeName == "Mesh" }
            ?: return Report(
                Kind.INCOMPLETE_OR_UNKNOWN, emptySet(), false,
                "The SDNA schema has no Mesh structure."
            )
        val fields = mesh.fields.mapNotNull { field ->
            identifier(field.fieldName)
        }.toSet()
        val hasCustomDataTypes = schema.structs.any { it.typeName == "CustomData" } &&
            schema.structs.any { it.typeName == "CustomDataLayer" }

        val legacyFields = setOf("mvert", "mloop", "mpoly", "totvert", "totloop", "totpoly")
        if (legacyFields.all { it in fields }) {
            return Report(
                Kind.LEGACY_DIRECT_ARRAYS, fields, hasCustomDataTypes,
                "Mesh exposes direct vertex/loop/polygon arrays; the legacy recognizer can attempt bounded decoding."
            )
        }

        val customDataFields = setOf("vdata", "edata", "fdata", "ldata", "pdata")
        if (hasCustomDataTypes && fields.any { it in customDataFields }) {
            return Report(
                Kind.CUSTOM_DATA_LAYOUT, fields, true,
                "Mesh uses a CustomData-style layout. Layer metadata is detected, but geometry remains unsupported until layer types and payloads are decoded."
            )
        }

        return Report(
            Kind.INCOMPLETE_OR_UNKNOWN, fields, hasCustomDataTypes,
            "Mesh fields do not match a known safe storage recognizer; geometry will not be guessed."
        )
    }

    private fun identifier(declaration: String): String? {
        val withoutArrays = declaration.substringBefore('[').trim()
        val match = Regex("[A-Za-z_][A-Za-z0-9_]*\\$").find(withoutArrays)
        return match?.value?.removeSuffix("$")
    }
}
