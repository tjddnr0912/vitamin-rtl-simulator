module slv (output reg rdy);
  assign rdy = 1'b1;                       // continuous assign to an output `reg` (its sole writer)
endmodule
module top;
  wire r;
  slv u (.rdy(r));
  initial begin
    top.g[0][1] = 32'd5;                   // self-hierarchical write to an unpacked-array element declared LATER
    #1 $display("r=%b g01=%0d", r, g[0][1]);
    $finish;
  end
  bit [31:0] [31:0] g [1];                 // declared after its hierarchical use (as dasm.svi does)
endmodule
