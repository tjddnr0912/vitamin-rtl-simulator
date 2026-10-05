module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  case (10)
    f(1): begin : g7
      initial begin #1 $display("gcl=label"); $finish; end
    end
    default: begin : gd
      initial begin #1 $display("gcl=default"); $finish; end
    end
  endcase
endmodule
