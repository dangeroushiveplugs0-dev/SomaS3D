package com.soma3d.app

/**
 * Safe lookup over Blender's stored datablock address ranges.
 *
 * These are file-format addresses only. They are never dereferenced as native pointers.
 * A range is indexed only when its start and end can be represented without overflow.
 */
class BlendBlockIndex(blocks: List<BlendBlockReader.BlockSummary>) {
    data class Entry(
        val block: BlendBlockReader.BlockSummary,
        val startAddress: Long,
        val endExclusive: Long
    )

    data class BuildResult(
        val index: BlendBlockIndex?,
        val rejectedBlocks: Int,
        val message: String
    )

    private val entries: List<Entry>
    private val starts: LongArray

    private constructor(entries: List<Entry>, marker: Unit) : this(emptyList()) {
        // The private constructor is intentionally not used; see create() below.
    }

    init {
        entries = emptyList()
        starts = LongArray(0)
    }

    private constructor(entries: List<Entry>) : this(emptyList()) {
        this.entries = entries
        this.starts = LongArray(entries.size) { entries[it].startAddress }
    }

    fun findContaining(fileAddress: Long): Entry? {
        if (fileAddress <= 0L || entries.isEmpty()) return null
        var low = 0
        var high = starts.size - 1
        var candidate = -1
        while (low <= high) {
            val mid = low + (high - low) / 2
            if (starts[mid] <= fileAddress) {
                candidate = mid
                low = mid + 1
            } else {
                high = mid - 1
            }
        }
        if (candidate < 0) return null
        val entry = entries[candidate]
        return entry.takeIf { fileAddress < it.endExclusive }
    }

    fun entries(): List<Entry> = entries

    companion object {
        fun create(blocks: List<BlendBlockReader.BlockSummary>): BuildResult {
            val candidates = ArrayList<Entry>()
            var rejected = 0
            for (block in blocks) {
                if (block.oldAddress <= 0L || block.payloadBytes <= 0L) continue
                if (block.oldAddress > Long.MAX_VALUE - block.payloadBytes) {
                    rejected++
                    continue
                }
                candidates.add(
                    Entry(block, block.oldAddress, block.oldAddress + block.payloadBytes)
                )
            }
            candidates.sortBy { it.startAddress }
            val nonOverlapping = ArrayList<Entry>(candidates.size)
            var previousEnd = 0L
            for (entry in candidates) {
                if (nonOverlapping.isNotEmpty() && entry.startAddress < previousEnd) {
                    rejected++
                    continue
                }
                nonOverlapping.add(entry)
                previousEnd = entry.endExclusive
            }
            val index = BlendBlockIndex(nonOverlapping)
            val message = if (rejected == 0) {
                "Indexed ${nonOverlapping.size} non-empty datablock address ranges."
            } else {
                "Indexed ${nonOverlapping.size} ranges; rejected ${rejected} invalid or overlapping ranges."
            }
            return BuildResult(index, rejected, message)
        }
    }
}
