module tap (input logic tck, input logic trst, output logic [3:0] state);
  always @(posedge tck or negedge trst)     // async reset seen only through an edge
    if (!trst) state <= 4'h0; else state <= state + 4'h1;
endmodule
module top;
  logic [3:0] s;
  tap u (.tck(1'b0), .trst(1'b0), .state(s)); // reset tied to a constant 0: its only edge is the time-0 x/z->0 change
  initial #1 begin $display("state=%b", s); $finish; end
endmodule
