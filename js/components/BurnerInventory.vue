<script>
import fuelBack from "../../img/fuel-back.png";

export default {
  name: 'BurnerInventory',
  props: {
    items: Array,
    burnerEnergy: Number,
  },
  emits: ['clickFuel', 'mouseEnter', 'mouseLeave'],

  setup(props, context) {
    return {
      fuelBack,
      onClickFuel: (i, evt, rightClick) => context.emit("clickFuel", i, evt, rightClick),
      mouseEnter: (...args) => context.emit("mouseEnter", ...args),
      mouseLeave: (...args) => context.emit("mouseLeave", ...args),
    }
  }
}
</script>

<template>
  <div class="itemBack"
    @click="evt => onClickFuel(0, evt, false)"
    @contextmenu="evt => onClickFuel(0, evt, true)"
    @mouseenter="evt => mouseEnter(0, evt)"
    @mouseleave="evt => mouseLeave(0, evt)"
    :style="{backgroundColor: `#ffffff`, backgroundImage: `url(${fuelBack})`}"
  >
    <img v-if="0 < items.length" class="burnerItem" :src="items[0].url">
    <div v-if="0 < items.length" class="overlay noselect"> {{ items[0].count }} </div>
  </div>
  <div class="burnerEnergyBack">
    <div id="burnerEnergy" class="burnerEnergy" :style="{width: `${burnerEnergy * 100}%`}"></div>
  </div>
</template>
