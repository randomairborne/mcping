package main

import (
	"encoding/json"
	"log"
	"math"
	"net/http"
	"os"
	"strconv"
	"strings"

	"github.com/iverly/go-mcping/mcping"
)

type RootHandler struct {
	root []byte
}

type ApiHandler struct{}

type IconHandler struct {
	icon []byte
}

type ApiError struct {
	Error string `json:"error"`
}

type PlayerSample struct {
	UUID string `json:"uuid"`
	Name string `json:"playername"`
}

type Players struct {
	Online  int            `json:"online"`
	Maximum int            `json:"maximum"`
	Sample  []PlayerSample `json:"sample"`
}

type Version struct {
	Protocol  int    `json:"protocol"`
	Broadcast string `json:"broadcast"`
}

type ApiResponse struct {
	Latency uint    `json:"latency"`
	Players Players `json:"players"`
	MOTD    string  `json:"motd"`
	Icon    string  `json:"icon"`
	Version Version `json:"version"`
}

const jsonMarshalError = `{"error": "Error marshaling json! Please make a bug report: https://github.com/randomairborne/mcping/issues"}`

func main() {
	staticpage, err := os.ReadFile("ping.html")
	if err != nil {
		log.Fatal(err)
		return
	}
	icon, err := os.ReadFile("icon.png")
	if err != nil {
		log.Fatal(err)
		return
	}
	mux := http.NewServeMux()
	mux.Handle("/", &RootHandler{staticpage})
	mux.Handle("/api/", &ApiHandler{})
	mux.Handle("/icon.png", &IconHandler{icon})
	log.Println("Listening on port 8080")
	err = http.ListenAndServe(":8080", mux)
	if err != nil {
		log.Fatal(err)
	}
}

func (h *RootHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html; charset=utf-8")
	w.Write(h.root)
}

func (h *IconHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "image/png")
	w.Write(h.icon)
}

func (h *ApiHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	w.Header().Add("Content-Type", "application/json")
	path := strings.Split(r.URL.Path, "/")
	address := path[len(path)-1]
	connectionSlice := strings.Split(address, ":")
	port := 25565
	ip := connectionSlice[0]
	if len(connectionSlice) == 2 {
		port, err := strconv.Atoi(connectionSlice[1])
		if err != nil {
			failure(w, "Port value is not a number!")
			return
		}
		if port > math.MaxUint16 {
			failure(w, "Port integer is not a valid port!")
			return
		}
	}
	pinger := mcping.NewPinger()
	result, err := pinger.Ping(ip, uint16(port))
	if err != nil {
		failure(w, "Failed to connect to server: "+err.Error())
		return
	}
	playerSamples := []PlayerSample{}
	for _, player := range result.Sample {
		playerSamples = append(playerSamples, PlayerSample{
			UUID: player.UUID,
			Name: player.Name,
		})
	}
	jsonResult := ApiResponse{
		Latency: result.Latency,
		MOTD:    result.Motd,
		Version: Version{
			Protocol:  result.Protocol,
			Broadcast: result.Version,
		},
		Players: Players{
			Maximum: result.PlayerCount.Max,
			Online:  result.PlayerCount.Online,
			Sample:  playerSamples,
		},
		Icon: result.Favicon,
	}
	response, err := json.Marshal(jsonResult)
	if err != nil {
		failure(w, "Failed to marshal response JSON!")
		return
	}
	w.Write(response)
}

func failure(w http.ResponseWriter, e string) {
	failure, err := json.Marshal(ApiError{Error: e})
	w.Header().Set("Content-Type", "application/json")
	if err != nil {
		w.Write([]byte(jsonMarshalError))
	}
	w.Write(failure)
}