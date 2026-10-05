module top;
  function automatic integer f(input integer x); f = x + 100; endfunction
  if (1) begin : g
    if (f(1) == 2) begin : a
      wire [7:0] w = 8'd2;
      initial #1 $display("@inner w=%0d bits=%0d", w, $bits(w));
    end else begin : b
      wire [3:0] w = 4'd1;
      initial #1 $display("@outer w=%0d bits=%0d", w, $bits(w));
    end
    function automatic integer f(input integer x); f = x + 1; endfunction
  end
  initial #10 $finish;
endmodule
