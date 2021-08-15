const path = require("path");
const CopyPlugin = require("copy-webpack-plugin");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");
const HtmlWebpackPlugin = require('html-webpack-plugin');
const { VueLoaderPlugin } = require('vue-loader');

const dist = path.resolve(__dirname, "dist");

module.exports = {
  mode: "production",
  entry: {
    index: "./js/index.js"
  },
  output: {
    path: dist,
    filename: "[name].js"
  },
  devServer: {
    contentBase: dist,
  },
  plugins: [
    // new CopyPlugin([
    //   path.resolve(__dirname, "static")
    // ]),

    new WasmPackPlugin({
      crateDirectory: __dirname,
    }),

    new HtmlWebpackPlugin({
      template: "./static/index.html"
    }),

    new VueLoaderPlugin({
      __VUE_OPTIONS_API__: true,
      __VUE_PROD_DEVTOOLS__: false
    }),
  ],
  module: {
    rules: [
      {
        test: /\.html$/i,
        loader: 'html-loader',
      },
      {
        test: /\.css$/i,
        use: ["file-loader"],
      },
      {
        test: /\.(png|jpe?g|gif)$/i,
        use: [
          {
            loader: 'url-loader',
            options: {
              limit: 8192,
            },
          },
        ],
      },
      {
        test: /\.vue$/,
        loader: 'vue-loader'
      },
    ]
  },
  resolve: {
    alias: {
      'vue$': "vue/dist/vue.esm-bundler.js"
    }
  }
};
