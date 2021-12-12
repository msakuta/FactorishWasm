<script>
import CloseButton from "./CloseButton.vue";
import { nextTick, ref, watch } from "vue";

export default {
  components: {
    CloseButton,
  },

  props: {
    onAltMode: Function,
    onShowDebugBBox: Function,
    onShowDebugFluidBox: Function,
    onShowDebugPowerNetwork: Function,
    onShowPerfGraph: Function,
    onUseWebGLInstancing: Function,
    dragWindowMouseDown: Function,
    bringToTop: Function,
  },

  setup(props) {
    const {
      onAltMode,
      onShowDebugBBox,
      onShowDebugFluidBox,
      onShowDebugPowerNetwork,
      onShowPerfGraph,
      onUseWebGLInstancing,
      dragWindowMouseDown,
    } = props;

    const visible = ref(false);

    const altMode = ref(false);
    const showDebugBBox = ref(false);
    const showDebugFluidBox = ref(false);
    const showDebugPowerNetwork = ref(false);
    const showPerfGraph = ref(false);
    const useWebGLInstancing = ref(true);

    watch(altMode, () => onAltMode(altMode.value));
    watch(showDebugBBox, () => onShowDebugBBox(showDebugBBox.value));
    watch(showDebugFluidBox, () => onShowDebugFluidBox(showDebugFluidBox.value));
    watch(showDebugPowerNetwork, () => onShowDebugPowerNetwork(showDebugPowerNetwork.value));
    watch(showPerfGraph, () => onShowPerfGraph(showPerfGraph.value));
    watch(useWebGLInstancing, () => onUseWebGLInstancing(useWebGLInstancing.value));

    return {
      visible,
      left: ref(0),
      top: ref(0),

      zIndex: ref(0),
      dragWindowMouseDown,

      altMode,
      showDebugBBox,
      showDebugFluidBox,
      showDebugPowerNetwork,
      showPerfGraph,
      useWebGLInstancing,

      inventoryDragStart: null,
      close(){
        console.log("Handling click: " + visible.value);
        visible.value = !visible.value;
      },
    };
  },

  methods: {
    dragWindow(evt){
      this.dragWindowMouseDown(
        evt,
        this.$refs.root,
        this,
        this.inventoryDragStart,
        (x, y) => {
          this.left = x;
          this.top = y;
        }
      )
    },

    // Place a window element at the center, for Vue component.
    placeCenter() {
      // Defer one tick to allow DOM element to be created
      nextTick(() => {
        if (!this.$refs.root) return;
        var elemRect = this.$refs.root.getBoundingClientRect();
        var bodyRect = document.body.getBoundingClientRect();
        this.left = (bodyRect.width - elemRect.width) / 2;
        this.top = (bodyRect.height - elemRect.height) / 2;
      });
    },
  },
};
</script>

<template>
  <div v-if="visible" ref="root"
    :class="['noselect', 'windowFrame']"
    :style="{left: `${left}px`, top: `${top}px`, zIndex}"
    @click="bringToTop"
  >
    <div class="inventoryTitle" @mousedown="dragWindow">New Game</div>
    <close-button @click="close"></close-button>
    <div style="padding: 10px">
      <div style="position: relative;">
        <div class="viewContainer">
          <div style="font-size: 120%; font-weight: 700">View settings</div>
          <div><label><input type="checkbox" v-model="altMode">Alt mode (alt key)</label></div>
          <div><label><input type="checkbox" v-model="showDebugBBox">Show Debug Bounding Box</label></div>
          <div><label><input type="checkbox" v-model="showDebugFluidBox">Show Debug Fluid Box</label></div>
          <div><label><input type="checkbox" v-model="showDebugPowerNetwork">Show Debug Power Network</label></div>
          <div><label><input type="checkbox" v-model="showPerfGraph">Show performance graph</label></div>
          <div><label><input type="checkbox" v-model="useWebGLInstancing" checked>Use WebGL instancing</label></div>
        </div>
      </div>
    </div>
  </div>
</template>
