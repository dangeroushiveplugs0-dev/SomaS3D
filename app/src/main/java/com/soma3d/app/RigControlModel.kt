package com.soma3d.app

/**
 * Normalized representation for rig controls imported from Blender.
 *
 * This model stores control definitions and links only. It deliberately does not
 * evaluate Blender driver expressions, execute scripts, or run Blender actions.
 * ShofterUI can map supported definitions to native, bounded controls.
 */
data class RigControlDefinition(
    val id: String,
    val label: String,
    val sourceKind: SourceKind,
    val valueKind: ValueKind,
    val defaultValue: Double,
    val minimum: Double?,
    val maximum: Double?,
    val description: String? = null
) {
    enum class SourceKind {
        OBJECT_CUSTOM_PROPERTY,
        BONE_CUSTOM_PROPERTY,
        CONTROL_BONE_TRANSFORM,
        ACTION_CONSTRAINT,
        TRANSFORMATION_CONSTRAINT,
        SHAPE_KEY
    }

    enum class ValueKind {
        BOOLEAN,
        INTEGER,
        FLOAT,
        VECTOR,
        ROTATION,
        ENUM,
        TEXT,
        UNKNOWN
    }
}

/**
 * A preserved relationship between a Blender control and the data it affects.
 * Expression text is archival metadata only; it must never be evaluated as code.
 */
data class RigControlLink(
    val sourceControlId: String,
    val targetId: String,
    val targetKind: TargetKind,
    val channel: String?,
    val expressionText: String? = null,
    val actionName: String? = null,
    val constraintType: String? = null,
    val supportedNatively: Boolean = false
) {
    enum class TargetKind {
        SHAPE_KEY,
        OBJECT_TRANSFORM,
        BONE_TRANSFORM,
        MATERIAL_PARAMETER,
        OTHER
    }
}

/**
 * Imported controls and links attached to the normalized character representation.
 * Unsupported links remain visible in the import report instead of being discarded.
 */
data class RigControlSet(
    val controls: List<RigControlDefinition>,
    val links: List<RigControlLink>,
    val warnings: List<String> = emptyList()
)
