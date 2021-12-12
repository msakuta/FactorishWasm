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
    onShowNewGame: Function,
    onShowViewSettings: Function,
    bringToTop: Function,
  },

  setup(props, context) {
    const {
      dragWindowMouseDown,
    } = props;

    const visible = ref(false);

    return {
      visible,
      left: ref(0),
      top: ref(0),

      zIndex: ref(0),
      dragWindowMouseDown,

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
    :style="{left: `${left}px`, top: `${top}px`, height: '120px', zIndex}"
    @click="bringToTop"
  >
    <div ref="recipeTitle" class="inventoryTitle" @mousedown="dragWindow">Main Menu</div>
    <close-button @click="close"></close-button>
    <div ref="recipeClient" :style="{
          fontSize: '120%',
          fontWeight: '700',
          textAlign: 'center',
          margin: '16px',
        }">
      <button @click="onShowNewGame" class="largeButton">New game</button>
      <button @click="onShowViewSettings" class="largeButton">View Settings</button>
    </div>
  </div>
</template>
