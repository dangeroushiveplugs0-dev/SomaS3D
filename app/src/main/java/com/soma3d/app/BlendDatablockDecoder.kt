package com.soma3d.app

/**
 * Connects block metadata, cached payload reads, SDNA decoding, and file-address lookup.
 *
 * This stage produces conservative generic records; it does not yet interpret Blender
 * Object/Mesh semantics or execute stored driver/script content.
 */
class BlendDatablockDecoder(
    private val scan: BlendBlockReader.Result,
    private val source: BlendCachedFileSource,
    private val maxPayloadBytes: Int = 16 * 1024 * 1024,
    private val maxRecordsPerBlock: Int = 100_000
) {
    data class DecodedBlock(
        val block: BlendBlockReader.BlockSummary,
        val typeName: String?,
        val records: List<BlendStructDecoder.DecodedRecord>,
        val success: Boolean,
        val message: String
    )

    data class DecodeSummary(
        val blocks: List<DecodedBlock>,
        val decodedBlockCount: Int,
        val skippedBlockCount: Int,
        val failedBlockCount: Int,
        val message: String
    )

    private val addressIndex = BlendBlockIndex.create(scan.blocks)

    fun decodeBlock(block: BlendBlockReader.BlockSummary): DecodedBlock {
        if (!scan.validHeader || scan.schema == null || scan.pointerBits == null || scan.littleEndian == null) {
            return failure(block, null, "A valid header and SDNA schema are required.")
        }
        if (block.code == "DNA1" || block.code == "ENDB") {
            return failure(block, null, "Structural block is not a datablock record.")
        }
        val schema = scan.schema
        if (block.sdnaIndex < 0L || block.sdnaIndex >= schema.structs.size.toLong()) {
            return failure(block, null, "Block SDNA index is outside the schema.")
        }
        if (block.count < 0L || block.count > maxRecordsPerBlock.toLong()) {
            return failure(block, schema.structs[block.sdnaIndex.toInt()].typeName, "Block record count exceeds the safe decode limit.")
        }
        val payloadResult = source.readPayload(block, maxPayloadBytes)
        val payload = payloadResult.payload
            ?: return failure(block, schema.structs[block.sdnaIndex.toInt()].typeName, payloadResult.message)
        val decoded = BlendStructDecoder.decodeRecords(
            schema = schema,
            sdnaIndex = block.sdnaIndex,
            count = block.count,
            payload = payload,
            pointerBits = scan.pointerBits,
            littleEndian = scan.littleEndian,
            maxRecords = maxRecordsPerBlock
        )
        return DecodedBlock(block, schema.structs[block.sdnaIndex.toInt()].typeName,
            decoded.records, decoded.success, decoded.message)
    }

    /** Decodes only blocks whose SDNA types match the requested names. */
    fun decodeTypes(typeNames: Set<String>, maxBlocks: Int = 20_000): DecodeSummary {
        val schema = scan.schema
            ?: return DecodeSummary(emptyList(), 0, 0, 0, "SDNA schema is unavailable.")
        val candidates = scan.blocks.filter { block ->
            block.code != "DNA1" && block.code != "ENDB" &&
                block.sdnaIndex >= 0L && block.sdnaIndex < schema.structs.size.toLong() &&
                schema.structs[block.sdnaIndex.toInt()].typeName in typeNames
        }
        val selected = candidates.take(maxBlocks.coerceAtLeast(0))
        val results = selected.map { decodeBlock(it) }
        val successful = results.count { it.success }
        val failed = results.count { !it.success }
        val skipped = candidates.size - selected.size
        return DecodeSummary(
            results, successful, skipped, failed,
            "Decoded $successful requested-type block(s); $failed failed or unsupported; $skipped not attempted."
        )
    }

    /** Decodes blocks conservatively; malformed or unsupported blocks are reported individually. */
    fun decodeAll(maxBlocks: Int = 20_000): DecodeSummary {
        val candidates = scan.blocks.filter { it.code != "DNA1" && it.code != "ENDB" }
        val selected = candidates.take(maxBlocks.coerceAtLeast(0))
        val results = selected.map { decodeBlock(it) }
        val successful = results.count { it.success }
        val failed = results.count { !it.success }
        val skipped = (candidates.size - selected.size) + results.count {
            it.message == "Structural block is not a datablock record."
        }
        return DecodeSummary(
            results, successful, skipped, failed,
            "Decoded $successful block(s); $failed failed or unsupported; $skipped not attempted."
        )
    }

    fun findBlockContaining(fileAddress: Long): BlendBlockReader.BlockSummary? =
        addressIndex.index.findContaining(fileAddress)?.block

    private fun failure(
        block: BlendBlockReader.BlockSummary,
        typeName: String?,
        message: String
    ) = DecodedBlock(block, typeName, emptyList(), false, message)
}
