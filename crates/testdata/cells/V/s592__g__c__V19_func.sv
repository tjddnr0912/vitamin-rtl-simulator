module top;
  if (1) begin : gb
    case (8'd99)
      K: begin : g function automatic [7:0] f(input [7:0] a); f = a + 8'd100; endfunction
                   initial #1 $display("@k f=%0d", f(8'd1)); end
      default: begin : g function automatic [7:0] f(input [7:0] a); f = a + 8'd7; endfunction
                   initial #1 $display("@def f=%0d", f(8'd1)); end
    endcase
    localparam logic [7:0] K = 8'd99;
  end
  initial #5 $finish;
endmodule
