`timescale 1ns/1ns
module s #(parameter P = 4'sb1111); case (P) -1: begin : a initial $display("O15 item"); end default: begin : d initial $display("O15 default"); end endcase endmodule
module t; s #(.P(4'd15)) u(); endmodule
