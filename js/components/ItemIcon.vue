<script>
import { getImageFile } from "../images";

export default {
  name: 'ItemIcon',
  props: {
    item: String,
    count: {
      type: Number,
      default: 0,
    },
    noCount: {
      type: Boolean,
      default: false,
    },
    transparent: {
      type: Boolean,
      default: false,
    },
    spoil: {
      type: Number,
      default: 0,
    }
  },

  setup(props, context) {
    return {
      style: () => {
        const itemFile = getImageFile(props.item);
        return {
          display: `inline-block`,
          position: "relative",
          backgroundImage: `url(${itemFile.url})`,
          backgroundSize: 32 * itemFile.widthFactor + 'px ' + 32 * itemFile.heightFactor + 'px',
          width: `32px`,
          height: `32px`,
        };
      },
      spoilBar: () => ({
        width: `${props.spoil * 32}px`,
        position: "absolute",
        left: "0px",
        bottom: "0px",
        height: "4px",
        backgroundColor: "#fff",
      }),
    }
  }
}
</script>

<template>
  <div :style="style()" :class="[count === 0 && !noCount || transparent ? 'transparent' : '']">
    <div v-if="!noCount && 0 < count" class="overlay noselect">
    {{ count }}
    </div>
    <div v-if="spoil !== 0" :style="spoilBar()"></div>
  </div>
</template>
