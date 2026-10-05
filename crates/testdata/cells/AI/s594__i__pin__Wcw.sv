module top;
localparam bit [7:0] CA [2] = '{8'd200, 8'd100};
logic [63:0] v;
initial begin
  v = 64'hffff_ffff_ffff_ffff;
  $display("CW=%h", v[0 +: (CA[0] + CA[1])]);
end
initial #5 $finish;
endmodule
