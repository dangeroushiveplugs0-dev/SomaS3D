package com.soma3d.app

/**
 * Safely inspects CustomData layer descriptors embedded in Mesh records.
 * This inventories metadata only; it does not guess payload layouts or execute stored behavior.
 */
object BlendCustomDataInspector {
    private const val MAX_LAYERS_PER_DOMAIN = 256
    private const val MAX_TOTAL_LAYERS = 1024

    data class LayerInfo(val domain: String, val name: String, val typeId: Int?, val hasDataAddress: Boolean)
    data class DomainInfo(val domain: String, val declaredLayerCount: Int, val decodedLayerCount: Int, val warning: String? = null)
    data class Result(
        val inspectedMeshCount: Int,
        val layers: List<LayerInfo>,
        val domains: List<DomainInfo>,
        val warnings: List<String>,
        val message: String
    )

    private val domains = listOf(
        "vdata" to "vertex", "edata" to "edge", "ldata" to "corner",
        "pdata" to "face", "fdata" to "legacy-face"
    )

    fun inspect(meshRecords: List<BlendStructDecoder.DecodedRecord>, decoder: BlendDatablockDecoder): Result {
        val layers = ArrayList<LayerInfo>()
        val domainInfos = ArrayList<DomainInfo>()
        val warnings = ArrayList<String>()
        var inspectedMeshes = 0

        for ((meshIndex, mesh) in meshRecords.withIndex()) {
            val meshName = (mesh.fields["name"] as? String)?.takeIf { it.isNotBlank() } ?: "Mesh ${meshIndex + 1}"
            var meshHadDomain = false
            for ((fieldName, domainName) in domains) {
                val customData = mesh.fields[fieldName] as? Map<*, *> ?: continue
                meshHadDomain = true
                val count = readCount(customData["totlayer"])
                val pointer = readPointer(customData["layers"])
                if (count == null) {
                    val warning = "$meshName: $domainName CustomData layer count is missing or invalid."
                    warnings.add(warning)
                    domainInfos.add(DomainInfo(domainName, 0, 0, warning))
                    continue
                }
                if (count == 0) {
                    domainInfos.add(DomainInfo(domainName, 0, 0))
                    continue
                }
                if (count > MAX_LAYERS_PER_DOMAIN || layers.size + count > MAX_TOTAL_LAYERS) {
                    val warning = "$meshName: $domainName declares too many CustomData layers; inventory limit applied."
                    warnings.add(warning)
                    domainInfos.add(DomainInfo(domainName, count, 0, warning))
                    continue
                }
                if (pointer == null || pointer <= 0L) {
                    val warning = "$meshName: $domainName has $count layer(s) but no valid layer-array address."
                    warnings.add(warning)
                    domainInfos.add(DomainInfo(domainName, count, 0, warning))
                    continue
                }
                val decoded = decoder.decodeRecordsAtAddress(pointer, "CustomDataLayer", count)
                if (!decoded.success || decoded.records.size != count) {
                    val warning = "$meshName: $domainName layer descriptors could not be decoded safely (${decoded.message})."
                    warnings.add(warning)
                    domainInfos.add(DomainInfo(domainName, count, 0, warning))
                    continue
                }
                val mapped = summarizeLayerRecords(domainName, decoded.records)
                layers.addAll(mapped)
                domainInfos.add(DomainInfo(domainName, count, mapped.size))
            }
            if (meshHadDomain) inspectedMeshes++
        }
        val message = "Inspected CustomData metadata for $inspectedMeshes mesh(es); found ${layers.size} layer descriptor(s), ${warnings.size} warning(s)."
        return Result(inspectedMeshes, layers, domainInfos, warnings, message)
    }

    /** Pure mapping helper for unit-testing partial or malformed layer descriptors. */
    fun summarizeLayerRecords(domain: String, records: List<BlendStructDecoder.DecodedRecord>): List<LayerInfo> =
        records.map { record ->
            LayerInfo(
                domain = domain,
                name = (record.fields["name"] as? String)?.trim()?.takeIf { it.isNotEmpty() } ?: "(unnamed layer)",
                typeId = readCount(record.fields["type"]),
                hasDataAddress = (readPointer(record.fields["data"]) ?: 0L) > 0L
            )
        }

    private fun readCount(value: Any?): Int? {
        val number = when (value) {
            is Byte -> value.toLong()
            is Short -> value.toLong()
            is Int -> value.toLong()
            is Long -> value
            else -> return null
        }
        return number.takeIf { it in 0L..Int.MAX_VALUE.toLong() }?.toInt()
    }

    private fun readPointer(value: Any?): Long? = when (value) {
        is Int -> value.toLong()
        is Long -> value
        else -> null
    }
}
