<script>
import fuelBack from "../../img/fuel-back.png";

export default {
  name: 'BurnerInventory',
  props: {
    items: Array,
    burnerEnergy: Number,
  },
  emits: ['clickFuel'],

  setup(props, context) {
    return {
      fuelBack,
      onClickFuel: (i) => context.emit("clickFuel", i),
    }
  }
}
</script>

<template>
  <div class="itemBack" @click="onClickFuel(0)" :style="{backgroundColor: `#ffffff`, backgroundImage: `url(${fuelBack})`}">
    <img v-if="0 < items.length" class="burnerItem" :src="items[0].url">
    <div v-if="0 < items.length" class="overlay noselect"> {{ items[0].count }} </div>
  </div>
  <div class="burnerEnergyBack">
    <div id="burnerEnergy" class="burnerEnergy" :style="{width: `${burnerEnergy * 100}%`}"></div>
  </div>
</template>
