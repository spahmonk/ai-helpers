// Package signatures provides comprehensive Go code for testing signature extraction.
package signatures

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"sync"
)

// Config represents application configuration.
type Config struct {
	Name    string
	Version string
	Debug   bool
	Timeout int
}

// NewConfig creates a new configuration instance.
func NewConfig(name, version string) *Config {
	return &Config{
		Name:    name,
		Version: version,
		Debug:   false,
		Timeout: 30,
	}
}

// Validate checks if configuration is valid.
func (c *Config) Validate() error {
	if c.Name == "" {
		return errors.New("name cannot be empty")
	}
	if c.Version == "" {
		return errors.New("version cannot be empty")
	}
	return nil
}

// SetDebug enables or disables debug mode.
func (c *Config) SetDebug(enabled bool) {
	c.Debug = enabled
}

// Processor defines the interface for data processing.
type Processor interface {
	Process(data string) (string, error)
	Validate(data string) bool
	Name() string
}

// BaseProcessor provides common processor functionality.
type BaseProcessor struct {
	config *Config
	cache  map[string]string
	mu     sync.RWMutex
}

// NewBaseProcessor creates a new base processor.
func NewBaseProcessor(config *Config) *BaseProcessor {
	return &BaseProcessor{
		config: config,
		cache:  make(map[string]string),
	}
}

// GetCached retrieves a cached value.
func (bp *BaseProcessor) GetCached(key string) (string, bool) {
	bp.mu.RLock()
	defer bp.mu.RUnlock()
	value, ok := bp.cache[key]
	return value, ok
}

// SetCached stores a value in cache.
func (bp *BaseProcessor) SetCached(key, value string) {
	bp.mu.Lock()
	defer bp.mu.Unlock()
	bp.cache[key] = value
}

// ClearCache clears the cache.
func (bp *BaseProcessor) ClearCache() {
	bp.mu.Lock()
	defer bp.mu.Unlock()
	bp.cache = make(map[string]string)
}

// TextProcessor handles text data.
type TextProcessor struct {
	*BaseProcessor
}

// NewTextProcessor creates a text processor.
func NewTextProcessor(config *Config) *TextProcessor {
	return &TextProcessor{
		BaseProcessor: NewBaseProcessor(config),
	}
}

// Process processes text data.
func (tp *TextProcessor) Process(data string) (string, error) {
	if cached, ok := tp.GetCached(data); ok {
		return cached, nil
	}
	result := fmt.Sprintf("[TEXT] %s", data)
	tp.SetCached(data, result)
	return result, nil
}

// Validate checks if data is valid text.
func (tp *TextProcessor) Validate(data string) bool {
	return len(data) > 0
}

// Name returns processor name.
func (tp *TextProcessor) Name() string {
	return "TextProcessor"
}

// JSONProcessor handles JSON data.
type JSONProcessor struct {
	*BaseProcessor
}

// NewJSONProcessor creates a JSON processor.
func NewJSONProcessor(config *Config) *JSONProcessor {
	return &JSONProcessor{
		BaseProcessor: NewBaseProcessor(config),
	}
}

// Process processes JSON data.
func (jp *JSONProcessor) Process(data string) (string, error) {
	var obj interface{}
	if err := json.Unmarshal([]byte(data), &obj); err != nil {
		return "", err
	}
	formatted, err := json.MarshalIndent(obj, "", "  ")
	return string(formatted), err
}

// Validate checks if data is valid JSON.
func (jp *JSONProcessor) Validate(data string) bool {
	var obj interface{}
	return json.Unmarshal([]byte(data), &obj) == nil
}

// Name returns processor name.
func (jp *JSONProcessor) Name() string {
	return "JSONProcessor"
}

// ProcessorFactory creates processor instances.
type ProcessorFactory struct {
	processors map[string]func(*Config) Processor
}

// NewProcessorFactory creates a new factory.
func NewProcessorFactory() *ProcessorFactory {
	return &ProcessorFactory{
		processors: make(map[string]func(*Config) Processor),
	}
}

// RegisterProcessor registers a processor type.
func (pf *ProcessorFactory) RegisterProcessor(name string, factory func(*Config) Processor) {
	pf.processors[name] = factory
}

// CreateProcessor creates a processor of specified type.
func (pf *ProcessorFactory) CreateProcessor(name string, config *Config) (Processor, error) {
	factory, ok := pf.processors[name]
	if !ok {
		return nil, fmt.Errorf("unknown processor type: %s", name)
	}
	return factory(config), nil
}

// ApplicationManager manages application instances.
type ApplicationManager struct {
	config      *Config
	processors  map[string]Processor
	factory     *ProcessorFactory
	mu          sync.RWMutex
}

// NewApplicationManager creates a new application manager.
func NewApplicationManager(config *Config) *ApplicationManager {
	am := &ApplicationManager{
		config:     config,
		processors: make(map[string]Processor),
		factory:    NewProcessorFactory(),
	}
	am.factory.RegisterProcessor("text", func(c *Config) Processor { return NewTextProcessor(c) })
	am.factory.RegisterProcessor("json", func(c *Config) Processor { return NewJSONProcessor(c) })
	return am
}

// RegisterProcessor registers a custom processor.
func (am *ApplicationManager) RegisterProcessor(name string, processor Processor) {
	am.mu.Lock()
	defer am.mu.Unlock()
	am.processors[name] = processor
}

// ProcessFile processes a file with specified processor.
func (am *ApplicationManager) ProcessFile(filename, processorName string) (string, error) {
	am.mu.RLock()
	processor, ok := am.processors[processorName]
	am.mu.RUnlock()

	if !ok {
		return "", fmt.Errorf("unknown processor: %s", processorName)
	}

	data, err := os.ReadFile(filename)
	if err != nil {
		if am.config.Debug {
			fmt.Printf("Error reading %s: %v\n", filename, err)
		}
		return "", err
	}

	return processor.Process(string(data))
}

// GetConfig returns the configuration.
func (am *ApplicationManager) GetConfig() *Config {
	return am.config
}

// EncodeString encodes a string to hexadecimal.
func EncodeString(text string) string {
	result := ""
	for _, ch := range text {
		result += fmt.Sprintf("%x", ch)
	}
	return result
}

// DecodeString decodes hexadecimal string.
func DecodeString(encoded string) (string, error) {
	result := ""
	for i := 0; i < len(encoded); i += 2 {
		var code int
		if _, err := fmt.Sscanf(encoded[i:i+2], "%x", &code); err != nil {
			return "", err
		}
		result += string(rune(code))
	}
	return result, nil
}

func main() {
	config := NewConfig("demo", "1.0")
	config.SetDebug(true)
	manager := NewApplicationManager(config)
	fmt.Println("Go Application initialized")
}
