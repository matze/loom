const App = {
  data() {
    return {
      current: fetch("/api/current")
        .then(response => response.json())
        .then(data => (this.current = data.point))
    }
  },
  methods: {
    increase() {
      this.current = Math.round((this.current + 0.1) * 10) / 10
      this.update(null)
    },
    decrease() {
      this.current = Math.round((this.current - 0.1) * 10) / 10
      this.update(null)
    },
    update(event) {
      this.current = parseFloat(this.current)

      const options = {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ point: this.current })
      }

      fetch("/api/current", options)
        .then(response => response)
    }
  }
}

Vue.createApp(App).mount("#app")

fetch("/api/series")
  .then(response => response.json())
  .then(
    function(series) {
      var plotData = [
        {
          x: series.raw.dates,
          y: series.raw.weights,
          type: "scatters",
          mode: "markers",
          name: "Raw data",
        },
        {
          x: series.average.dates,
          y: series.average.weights,
          type: "scatters",
          name: "Average",
        }
      ]

      var layout = {
        xaxis: {
          rangeselector: {buttons: [
            {
              count: 1,
              label: "1m",
              step: "month",
              stepmode: "backward"
            },
            {
              count: 6,
              label: "6m",
              step: "month",
              stepmode: "backward"
            },
            {
              step: "all"
            }
          ]},
          type: "date"
        },
        legend: {
          x: 1,
          xanchor: "right",
          y: 1,
        },
      }

      var div = document.getElementById("plot")
      Plotly.newPlot(div, plotData, layout)
    }
  )
