module top; logic c; logic [99:0] a; sequence s; a <= 'z; endsequence assert property (@(posedge c) s); endmodule
