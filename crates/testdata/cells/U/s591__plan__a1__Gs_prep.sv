module top;
  localparam string i = "AB";
  for (genvar i = 0; i < 2; i++) begin : g
    wire [7:0] r = {(i+1){1'b1}};
    localparam [7:0] RR = {(i+1){1'b1}};
    initial #3 $display("prep %m r=%b RR=%b b=%0d", r, RR, $bits(i));
  end
  initial #100 $finish;
endmodule
