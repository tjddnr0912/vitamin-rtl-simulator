class C; rand logic [99:0] a; constraint k { a <= 'z; } endclass module top; C o; initial o = new; endmodule
