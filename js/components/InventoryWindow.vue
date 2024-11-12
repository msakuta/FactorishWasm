<script>
import CloseButton from "./CloseButton.vue";
import ItemIcon from "./ItemIcon.vue";
import BurnerInventory from "./BurnerInventory.vue";
import itemBack from "../../img/item-back.png";
import { nextTick, ref } from "vue";

export default {
  components: {
    CloseButton,
    BurnerInventory,
    ItemIcon,
  },

  props: {
    dragWindowMouseDown: Function,
    inventoryClickHandler: Function,
    inventoryMouseEnterHandler: Function,
    inventoryMouseLeaveHandler: Function,
    playerClickHandler: Function,
    playerMouseEnterHandler: Function,
    playerMouseLeaveHandler: Function,
    showRecipeSelect: Function,
    recipeSelectMouseEnterHandler: Function,
    recipeSelectMouseLeaveHandler: Function,
    bringToTop: Function,
  },

  setup(props, context) {
    const {
      dragWindowMouseDown,
      inventoryClickHandler,
      inventoryMouseEnterHandler,
      inventoryMouseLeaveHandler,
      playerClickHandler,
      playerMouseEnterHandler,
      playerMouseLeaveHandler,
    } = props;

    const inventoryVisible = ref(false);
    const burnerItems = ref([]);
    const inputItems = ref([]);
    const outputItems = ref([]);
    const storageItems = ref([]);
    const playerItems = ref([]);
    const onClose = ref(() => {});

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
      hasOutput: ref(false),
      outputItems,
      hasStorage: ref(false),
      storageItems,
      progress: ref(0),
      playerItems,

      zIndex: ref(0),
      dragWindowMouseDown,

      inventoryDragStart: null,
      onClose,
      close(){
        console.log("Handling click: " + inventoryVisible.value);
        inventoryVisible.value = !inventoryVisible.value;
        onClose.value(inventoryVisible.value);
      },

      onClickFuel: inventoryClickHandler(() => burnerItems.value, "Burner"),
      onMouseEnterFuel: inventoryMouseEnterHandler(() => burnerItems.value, "Burner"),
      onMouseLeaveFuel: inventoryMouseLeaveHandler(() => burnerItems.value, "Burner"),
      onClickInput: inventoryClickHandler(() => inputItems.value.value, "Input"),
      onMouseEnterInput: inventoryMouseEnterHandler(() => inputItems.value.value, "Input"),
      onMouseLeaveInput: inventoryMouseLeaveHandler(() => inputItems.value.value, "Input"),
      onClickOutput: inventoryClickHandler(() => outputItems.value.value, "Output"),
      onMouseEnterOutput: inventoryMouseEnterHandler(() => outputItems.value.value, "Output"),
      onMouseLeaveOutput: inventoryMouseLeaveHandler(() => outputItems.value.value, "Output"),
      onClickStorage: inventoryClickHandler(() => storageItems.value.value, "Storage"),
      onMouseEnterStorage: inventoryMouseEnterHandler(() => storageItems.value.value, "Storage"),
      onMouseLeaveStorage: inventoryMouseLeaveHandler(() => storageItems.value.value, "Storage"),

      onClickPlayer: playerClickHandler,
      onMouseEnterPlayer: playerMouseEnterHandler,
      onMouseLeavePlayer: playerMouseLeaveHandler,
    };
  },

  methods: {
    dragWindow(evt){
      this.dragWindowMouseDown(
        evt,
        this.$refs.inventory,
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
        if (!this.$refs.inventory) return;
        var elemRect = this.$refs.inventory.getBoundingClientRect();
        var bodyRect = document.body.getBoundingClientRect();
        this.left = (bodyRect.width - elemRect.width) / 2;
        this.top = (bodyRect.height - elemRect.height) / 2;
      });
    },
  },
};
</script>

<template>
  <div v-if="inventoryVisible" ref="inventory"
    :class="['noselect', 'inventory', hasPosition ? 'inventoryWide' : 'inventoryNarrow']"
    :style="{left: `${left}px`, top: `${top}px`, zIndex}"
    @click="bringToTop"
  >
    <div id="inventory2Title" @mousedown="dragWindow">Inventory</div>
    <close-button @click="close"></close-button>
    <div id="inventory2Client">
      <div v-if="hasPosition">
        <div style="vertical-align: middle;">
          <span v-if="hasInput">
            <div v-for="i in Math.max(1, inputItems.value.length)"
              :key="i"
              class="itemBack"
              @click="evt => onClickInput(i-1, evt, false)"
              @contextmenu="evt => onClickInput(i-1, evt, true)"
              @mouseenter="evt => onMouseEnterInput(i-1, evt)"
              @mouseleave="evt => onMouseLeaveInput(i-1, evt)"
              :style="{backgroundColor: `#ffffff`, backgroundImage: `url(${itemBack})`}"
            >
              <template v-if="i-1 < inputItems.value.length">
                <item-icon
                  :item="inputItems.value[i-1].name"
                  :count="inputItems.value[i-1].count"
                />
              </template>
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
              @click="evt => onClickOutput(i-1, evt, false)"
              @contextmenu="evt => onClickOutput(i-1, evt, true)"
              @mouseenter="evt => onMouseEnterOutput(i-1, evt)"
              @mouseleave="evt => onMouseLeaveOutput(i-1, evt)"
              :style="{backgroundColor: `#ffffff`, backgroundImage: `url(${itemBack})`}"
            >
              <template v-if="i-1 < outputItems.value.length">
                <item-icon
                  :item="outputItems.value[i-1].name"
                  :count="outputItems.value[i-1].count"
                />
              </template>
            </div>
          </span>
          <img
            v-if="hasInput && hasOutput"
            src="../../img/recipe-select.png"
            alt="Recipe select"
            @click="showRecipeSelect"
            @mouseenter="recipeSelectMouseEnterHandler"
            @mouseleave="recipeSelectMouseLeaveHandler"
          >
        </div>
        <burner-inventory
          v-if="hasBurner"
          @click-fuel="onClickFuel"
          @mouse-enter="onMouseEnterFuel"
          @mouse-leave="onMouseLeaveFuel"
          :items="burnerItems"
          :burnerEnergy="burnerEnergy"
        ></burner-inventory>
        <div v-if="hasStorage">
            <div class="inventorySubTitle">Storage inventory</div>
            <div v-for="i in 48"
              :key="i"
              class="itemBack"
              @click="evt => onClickStorage(i-1, evt, false)"
              @contextmenu="evt => onClickStorage(i-1, evt, true)"
              @mouseenter="evt => onMouseEnterStorage(i-1, evt)"
              @mouseleave="evt => onMouseLeaveStorage(i-1, evt)"
              :style="{backgroundColor: `#ffffff`, backgroundImage: `url(${itemBack})`}"
            >
              <template v-if="i-1 < storageItems.value.length">
                <item-icon
                  :item="storageItems.value[i-1].name"
                  :count="storageItems.value[i-1].count"
                />
              </template>
            </div>
        </div>
      </div>
      <div class="player" :style="{left: hasPosition ? '330px' : '0px'}">
        <div class="inventorySubTitle">Player inventory</div>
        <div class="playerInventoryContainer">
            <div v-for="i in Math.ceil((1 + playerItems.value.length) / 10) * 10"
              :key="i"
              class="itemBack"
              @click="evt => onClickPlayer(i-1, evt, false)"
              @contextmenu="evt => onClickPlayer(i-1, evt, true)"
              @mouseenter="evt => onMouseEnterPlayer(i-1, evt)"
              @mouseleave="evt => onMouseLeavePlayer(i-1, evt)"
              :style="{backgroundColor: `#ffffff`, backgroundImage: `url(${itemBack})`}"
            >
              <template v-if="i-1 < playerItems.value.length">
                <item-icon
                  :item="playerItems.value[i-1].name"
                  :count="playerItems.value[i-1].count"
                  :spoil="playerItems.value[i-1].spoil"
                >
                </item-icon>
              </template>
            </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style>

</style>