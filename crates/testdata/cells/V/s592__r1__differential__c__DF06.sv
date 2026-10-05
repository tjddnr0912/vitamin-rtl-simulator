module top;
  function automatic integer f(input integer x); f = x + 100; endfunction
  if (1) begin : g
    case (f(1))
      2: begin : a initial $display("@inner"); end
      101: begin : b initial $display("@outer"); end
      default: begin : d initial $display("@def"); end
    endcase
    function automatic integer f(input integer x); f = x + 1; endfunction
  end
  initial #10 $finish;
endmodule
