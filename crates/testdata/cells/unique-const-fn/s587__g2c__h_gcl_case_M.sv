module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  case (7)
    f(2): begin : g7
      initial begin #1 $display("gcl=label"); $finish; end
    end
    default: begin : gd
      initial begin #1 $display("gcl=default"); $finish; end
    end
  endcase
endmodule
