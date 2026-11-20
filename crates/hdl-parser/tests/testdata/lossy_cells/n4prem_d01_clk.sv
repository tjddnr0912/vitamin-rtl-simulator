module top; logic c; logic [3:0] x; clocking cb @(posedge c); output o = 'x; endclocking endmodule
