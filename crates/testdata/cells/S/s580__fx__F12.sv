`timescale 1ns/1ns
module m #(parameter logic P = 1'b0) (); initial $display("override %b", P); endmodule
module t;
  localparam logic [3:0] PV = 4'b1100;
  localparam logic L1 = (4'b1100 inside {4'b1?00});
  localparam L2 = (4'b1000 inside {4'b1?00});
  localparam L4 = PV inside {4'b0?00};
  logic [(4'b1100 inside {4'b1?00}) : 0] wb;
  m #(.P(4'b1100 inside {4'b1?00})) u();
  if (PV inside {4'b1?00}) begin : g initial $display("gen-if then"); end
  else begin : g2 initial $display("gen-if else"); end
  for (genvar i = 0; i < 2; i++) begin : gl
    if ((PV + i) inside {4'b110?}) begin : h initial $display("gen-for %0d in", i); end
    else begin : k initial $display("gen-for %0d out", i); end
  end
  initial begin #1 $display("params %b %b %b bits %0d", L1, L2, L4, $bits(wb)); $finish; end
endmodule
