<script>
import CloseButton from "./CloseButton.vue";
import BurnerInventory from "./BurnerInventory.vue";
import itemBack from "../../img/item-back.png";
import { nextTick, reactive, ref, toRefs } from "vue";

export default {
  components: {
    CloseButton,
    BurnerInventory,
  },

  props: {
    dragWindowMouseDown: Function,
    showRecipeSelect: Function,
    inventoryClickHandler: Function,
    playerClickHandler: Function,
    windowOrder: Array,
  },

  setup(props, context) {
    const {
      dragWindowMouseDown,
      inventoryClickHandler,
      playerClickHandler,
      windowOrder
    } = props;

    const inventoryVisible = ref(false);
    const burnerItems = ref([]);
    const inputItems = ref([]);
    const outputItems = ref([]);
    const storageItems = ref([]);
    const playerItems = ref([]);

    return {
      inventoryVisible,
      left: ref(0),
      top: ref(0),
      hasPosition: ref(false),
      hasBurner: ref(false),
      burnerItems,
      burnerEnergy: ref(0),
      hasInput: ref(false),
      itemBack,
      inputItems,
      hasOutput: false,
      outputItems,
      hasStorage: false,
      storageItems,
      progress: ref(0),
      playerItems,

      windowOrder,
      dragWindowMouseDown,

      inventoryDragStart: null,
      close(){
        console.log("Handling click: " + inventoryVisible.value);
        inventoryVisible.value = !inventoryVisible.value;
      },

      showRecipeSelect: props.showRecipeSelect,

      onClickFuel: inventoryClickHandler(() => burnerItems.value, "Burner"),
      onClickInput: inventoryClickHandler(() => inputItems.value.value, "Input"),
      onClickOutput: inventoryClickHandler(() => outputItems.value.value, "Output"),
      onClickStorage: inventoryClickHandler(() => storageItems.value.value, "Storage"),

      onClickPlayer: playerClickHandler,
    };
  },

  methods: {
    dragWindow(evt){
      this.dragWindowMouseDown(
        evt,
        this.$refs.inventory,
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
        if (!this.$refs.inventory) return;
        var elemRect = this.$refs.inventory.getBoundingClientRect();
        var bodyRect = document.body.getBoundingClientRect();
        this.left = (bodyRect.width - elemRect.width) / 2;
        this.top = (bodyRect.height - elemRect.height) / 2;
        if (!(this.$refs.inventory in this.windowOrder))
          this.windowOrder.push(this.$refs.inventory);
      });
    },
  },
};
</script>

<template>
  <div v-if="inventoryVisible" ref="inventory"
  :class="['noselect', 'inventory', hasPosition ? 'inventoryWide' : 'inventoryNarrow']"
  :style="{left: `${left}px`, top: `${top}px`}"
  >
    <div id="inventory2Title" @mousedown="dragWindow">Inventory</div>
    <close-button @click="close"></close-button>
    <div id="inventoryButtons" class="inventoryButtons">
      <img id="inventory2List" draggable="false" src="../../img/list.png" style="position: absolute">
      <img id="inventory2Icons" draggable="false" src="../../img/icons.png" style="position: absolute; left: 0px; top: 32px">
      <img id="recipeSelectButton" draggable="false" src="../../img/recipe-select.png" alt="Recipe select"
          @click="showRecipeSelect"
          style="position: absolute; left: 0px; top: 80px; border: 1px solid #7f7f7f"
      >
    </div>
    <div id="inventory2Client">
      <div v-if="hasPosition">
        <burner-inventory v-if="hasBurner" @click-fuel="onClickFuel" :items="burnerItems" :burnerEnergy="burnerEnergy"></burner-inventory>
        <div style="vertical-align: middle;">
            <span v-if="hasInput">
                <div v-for="i in Math.max(1, inputItems.value.length)"
                    :key="i"
                    class="itemBack"
                    @click="onClickInput(i-1)"
                    :style="{backgroundColor: `#ffffff`, backgroundImage: `url(${itemBack})`}"
                >
                    <div v-if="i-1 < inputItems.value.length"
                        :class="['burnerItem', inputItems.value[i-1].count === 0 ? 'transparent' : '']"
                        :style="{backgroundImage: `url(${inputItems.value[i-1].url})`, backgroundSize: 32 * inputItems.value[i-1].widthFactor + 'px ' + 32 * inputItems.value[i-1].heightFactor + 'px'}">
                        <div v-if="i-1 < inputItems.value.length && 0 < inputItems.value[i-1].count" class="overlay noselect"> {{ inputItems.value[i-1].count }} </div>
                    </div>
                </div>
            </span>
            <span v-if="hasInput || hasOutput" style="position: relative; width: 100px;">
                <div class="progressBarBack">
                    <div class="progressBar" :style="{width: `${progress * 100}%`}"></div>
                </div>
            </span>
            <span v-if="hasOutput" style="position: relative">
                <div v-for="i in Math.max(1 + outputItems.value.length)"
                    :key="i"
                    class="itemBack"
                    @click="onClickOutput(i-1)"
                    :style="{backgroundColor: `#ffffff`, backgroundImage: `url(${itemBack})`}"
                >
                    <div v-if="i-1 < outputItems.value.length" class="burnerItem"
                        :style="{backgroundImage: `url(${outputItems.value[i-1].url})`, backgroundSize: 32 * outputItems.value[i-1].widthFactor + 'px ' + 32 * outputItems.value[i-1].heightFactor + 'px'}">
                        <div v-if="i-1 < outputItems.value.length" class="overlay noselect"> {{ outputItems.value[i-1].count }} </div>
                    </div>
                </div>
            </span>
        </div>
        <div v-if="hasStorage">
            <div class="inventoryTitle">Storage inventory</div>
            <div v-for="i in Math.ceil((1 + storageItems.value.length) / 10) * 10"
                :key="i"
                class="itemBack"
                @click="onClickStorage(i-1)"
                :style="{backgroundColor: `#ffffff`, backgroundImage: `url(${itemBack})`}"
            >
                <div v-if="i-1 < storageItems.value.length" class="burnerItem"
                    :style="{backgroundImage: `url(${storageItems.value[i-1].url})`, backgroundSize: 32 * storageItems.value[i-1].widthFactor + 'px ' + 32 * storageItems.value[i-1].heightFactor + 'px'}">
                    <div v-if="i-1 < storageItems.value.length" class="overlay noselect"> {{ storageItems.value[i-1].count }} </div>
                </div>
            </div>
        </div>
      </div>
      <div class="player" :style="{left: hasPosition ? '330px' : '0px'}">
        <div class="inventoryTitle">Player inventory</div>
        <div class="playerInventoryContainer">
            <div v-for="i in Math.ceil((1 + playerItems.value.length) / 10) * 10"
                :key="i"
                class="itemBack"
                @click="onClickPlayer(i-1)"
                :style="{backgroundColor: `#ffffff`, backgroundImage: `url(${itemBack})`}"
            >
                <div v-if="i-1 < playerItems.value.length" class="burnerItem"
                    :style="{backgroundImage: `url(${playerItems.value[i-1].url})`, backgroundSize: 32 * playerItems.value[i-1].widthFactor + 'px ' + 32 * playerItems.value[i-1].heightFactor + 'px'}">
                    <div v-if="i-1 < playerItems.value.length" class="overlay noselect"> {{ playerItems.value[i-1].count }} </div>
                </div>
            </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style>

</style>