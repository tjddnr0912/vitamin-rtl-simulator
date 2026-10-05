`timescale 1ns/1ns
module t;
  for (genvar i = 0; i < 2; i++) begin : g
    case (i - 1)
      32'hFFFFFFFF: begin : c initial $display("N09 item %0d", i); end
      default: begin : d initial $display("N09 default %0d", i); end
    endcase
    initial case (i - 1) 32'hFFFFFFFF: $display("N09p item %0d", i); default: $display("N09p default %0d", i); endcase
  end
endmodule
