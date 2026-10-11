package com.soma3d.app

/**
 * Version-aware names for CustomData type IDs verified against Blender's DNA_customdata_types.h.
 * This describes layer types only; it does not imply the layer payload has been decoded.
 */
object BlendCustomDataType {
    enum class Era { BLENDER_3_6, BLENDER_4_0 }

    fun label(typeId: Int?, blenderVersion: String?): String? {
        if (typeId == null) return null
        val versionNumber = blenderVersion?.takeIf { it.length >= 3 }?.take(3)?.toIntOrNull()
        val era = when (versionNumber) {
            in 300..399 -> Era.BLENDER_3_6
            in 400..499 -> Era.BLENDER_4_0
            else -> return null
        }
        return when (typeId) {
            2 -> "MDeformVert"
            4 -> "MFace"
            5 -> "MTFace"
            6 -> "MCol"
            7 -> "OrigIndex"
            8 -> "Normal"
            10 -> "Float"
            11 -> "Int32"
            12 -> "String"
            17 -> "ByteColor"
            19 -> "MultiresDisplacement"
            25 -> if (era == Era.BLENDER_3_6) "MPoly (deprecated)" else "Legacy MPoly"
            26 -> if (era == Era.BLENDER_3_6) "MLoop (deprecated)" else "Legacy MLoop"
            27 -> "ShapeKeyIndex"
            28 -> "ShapeKey"
            34 -> "PaintMask"
            45 -> "Int8"
            46 -> "Int32x2"
            47 -> "Color"
            48 -> "Float3"
            49 -> "Float2"
            50 -> "Boolean"
            51 -> if (era == Era.BLENDER_3_6) "HairLength" else "Unused in Blender 4.0"
            52 -> if (era == Era.BLENDER_4_0) "Quaternion" else null
            else -> null
        }
    }
}
