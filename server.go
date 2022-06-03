package main

import (
	"bytes"
	"encoding/base64"
	"encoding/json"
	"image/png"
	"log"
	"math"
	"net/http"
	"os"
	"strconv"
	"strings"
	"fmt"

	"github.com/iverly/go-mcping/mcping"
)

type RootHandler struct {
	root []byte
}

type ApiHandler struct{}

type IconHandler struct{}

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

type Icon struct {
	URL    string `json:"url"`
	Base64 string `json:"base64"`
}

type ApiResponse struct {
	Latency uint    `json:"latency"`
	Players Players `json:"players"`
	MOTD    string  `json:"motd"`
	Icon    Icon    `json:"icon"`
	Version Version `json:"version"`
}

const jsonMarshalError = `{"error": "Error marshaling json! Please make a bug report: https://github.com/randomairborne/mcpingme/issues"}`

func main() {
	staticpage, err := os.ReadFile("ping.html")
	if err != nil {
		log.Fatal(err)
	}
	mux := http.NewServeMux()
	mux.Handle("/", &RootHandler{staticpage})
	mux.Handle("/api/", &ApiHandler{})
	mux.Handle("/img/", &IconHandler{})
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
	faviconUrl := ""
	if result.Favicon != "" {
		faviconUrl = "https://mcping.me/img/" + address
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
		Icon: Icon{
			URL:    faviconUrl,
			Base64: result.Favicon,
		},
	}
	response, err := json.Marshal(jsonResult)
	if err != nil {
		failure(w, "Failed to marshal response JSON!")
		return
	}
	w.Write(response)
}

func (h *IconHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	w.Header().Add("Content-Type", "image/png")
	path := strings.Split(r.URL.Path, "/")
	address := path[len(path)-1]
	connectionSlice := strings.Split(address, ":")
	port := 25565
	ip := connectionSlice[0]
	if len(connectionSlice) == 2 {
		fmt.Printf("%#v", connectionSlice)
		port, err := strconv.Atoi(connectionSlice[1])
		if err != nil {
			failure(w, "Port value is not a number!")
			return
		}
		if port > math.MaxUint16 {
			failure(w, "Port integer is not a valid port!")
			return
		}
	} else if len(connectionSlice) > 2 {
		failure(w, "Invalid Server Address!")
		return
	}
	pinger := mcping.NewPinger()
	result, err := pinger.Ping(ip, uint16(port))
	if err != nil {
		failure(w, "Failed to connect to server: "+err.Error())
		return
	}
	if result.Favicon == "" {
		failure(w, "Server has no icon.")
		return
	}
	if !strings.HasPrefix(result.Favicon, "data:image/png;base64,") {
		failure(w, "Favicon was not base64!")
		return
	}
	favicon := strings.TrimPrefix(result.Favicon, "data:image/png;base64,")
	image, err := png.Decode(base64.NewDecoder(base64.StdEncoding, bytes.NewReader([]byte(favicon))))
	if err != nil {
		failure(w, "Failed to decode PNG: "+err.Error())
		return
	}
	buf := new(bytes.Buffer)
	err = png.Encode(buf, image)
	if err != nil {
		failure(w, "Failed to encode PNG: "+err.Error())
		return
	}
	data := buf.Bytes()
	w.Write(data)
}

func failure(w http.ResponseWriter, e string) {
	failure, err := json.Marshal(ApiError{Error: e})
	w.Header().Set("Content-Type", "application/json")
	if err != nil {
		w.Write([]byte(jsonMarshalError))
	}
	w.Write(failure)
}
