<script>
import fuelBack from "../../img/fuel-back.png";
import { nextTick, ref } from "vue";

export default {
  name: 'ToolTip',
  props: {
    visible: false,
    items: Array,
    burnerEnergy: Number,
  },

  setup(props, context) {
    return {
      visible: ref(false),
      tootipZIndex: ref(10000),
      fuelBack,
      left: ref(0),
      bottom: ref(0),
      title: ref(""),
      text: ref(""),
    }
  }
}
</script>

<template>
    <div v-if="visible" ref="tip" class="noselect tooltip"
      :style="{zIndex: tootipZIndex, left: left + 'px', bottom: bottom + 'px'}"
      v-html="text">
    </div>
</template>
