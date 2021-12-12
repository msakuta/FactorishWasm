<script>
import CloseButton from "./CloseButton.vue";
import ItemIcon from './ItemIcon.vue';
import { nextTick, ref } from "vue";

export default {
  components: {
    CloseButton,
    ItemIcon,
  },

  props: {
    dragWindowMouseDown: Function,
    onNewGame: Function,
    bringToTop: Function,
    defaultParams: Object,
  },

  setup(props) {
    const {
      dragWindowMouseDown,
      onNewGame,
      defaultParams,
    } = props;

    const visible = ref(false);

    const sizeSelect = ref(null);
    const scenarioSelect = ref(null);
    const terrainSeed = ref(defaultParams.terrainSeed);
    const waterNoiseThreshold = ref(defaultParams.waterNoiseThreshold);
    const resourceAmount = ref(defaultParams.resourceAmount);
    const noiseScale = ref(defaultParams.noiseScale);
    const noiseThreshold = ref(defaultParams.noiseThreshold);
    const noiseOctaves = ref(defaultParams.noiseOctaves);

    return {
      visible,
      left: ref(0),
      top: ref(0),

      zIndex: ref(0),
      dragWindowMouseDown,

      sizeSelect,
      scenarioSelect,
      terrainSeed,
      waterNoiseThreshold,
      resourceAmount,
      noiseScale,
      noiseThreshold,
      noiseOctaves,

      inventoryDragStart: null,
      close(){
        console.log("Handling click: " + visible.value);
        visible.value = !visible.value;
      },

      onNewGame: () => {
        visible.value = false;
        const sizeSelectElem = sizeSelect.value;
        const scenarioSelectElem = scenarioSelect.value;
        onNewGame({
          sizeStr: sizeSelectElem.value,
          scenario: scenarioSelectElem.value,
          terrainSeed: parseInt(terrainSeed.value),
          waterNoiseThreshold: parseFloat(waterNoiseThreshold.value),
          resourceAmount: parseFloat(resourceAmount.value),
          noiseScale: parseFloat(noiseScale.value),
          noiseThreshold: parseFloat(noiseThreshold.value),
          noiseOctaves: parseInt(noiseOctaves.value),
        });
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
    :class="['noselect', 'paramsContainer']"
    :style="{left: `${left}px`, top: `${top}px`, zIndex}"
    @click="bringToTop"
  >
    <div class="inventoryTitle" @mousedown="dragWindow">New Game</div>
    <close-button @click="close"></close-button>
    <div style="padding: 10px">
      <div style="font-size: 120%; font-weight: 700;">New game settings</div>
      <div>
        <label>Size:
          <select ref="sizeSelect">
            <option>16</option>
            <option>24</option>
            <option>32</option>
            <option>48</option>
            <option>64</option>
            <option>128</option>
            <option>256</option>
            <option selected>unlimited</option>
          </select>
        </label>
      </div>
      <div>
        <label>Scenario:
          <select ref="scenarioSelect">
            <option value="default" selected>Default</option>
            <option value="pipe_bench">Pipe benchmark</option>
            <option value="inserter_bench">Inserter benchmark</option>
            <option value="transport_bench">Transport belt benchmark</option>
            <option value="electric_bench">Electric network benchmark</option>
          </select>
        </label>
      </div>
      <div>
        Terrain Seed=<span ref="seedLabel"></span>
        <input type="number" v-model="terrainSeed">
      </div>
      <div>
        Water Noise Threshold:
        <input type="range" max="0.5" min="0" step="1e-3" v-model="waterNoiseThreshold">
        {{ waterNoiseThreshold }}
      </div>
      <div>
        Resource Amount:
        <input type="range" max="10000" min="100" step="100" v-model="resourceAmount">
        {{ resourceAmount }}
      </div>
      <div>
        Resource Noise Scale:
        <input type="range" max="20" min="2" step="1" v-model="noiseScale">
        {{ noiseScale }}
      </div>
      <div>
        Resource Noise threshold:
        <input type="range" max="0.5" min="0" step="1e-3" v-model="noiseThreshold">
        {{ noiseThreshold }}
      </div>
      <div>
        Noise Octaves:
        <input type="range" max="10" min="1" step="1" v-model="noiseOctaves">
        {{ noiseOctaves }}
      </div>
      <div style="text-align: center;">
        <button type="button" style="padding: 10px" @click="onNewGame">Start a new game!</button>
      </div>
    </div>
  </div>
</template>

<style>

</style>