module top; logic c; logic [99:0] a; property p; @(posedge c) a <= 'z; endproperty assert property (p); endmodule
