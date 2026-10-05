module top;
  logic [15:0] v;
  localparam logic [7:0] A = 8'd200;
  initial begin
    #10 $finish;
  end
  initial begin
    v = 16'h012C; $display("a %0d", v inside {8'd200 + 8'd100});
    v = 16'h002C; $display("b %0d", v inside {8'd200 + 8'd100});
    v = 16'h012C; $display("c %0d", v inside {A + 8'd100});
    v = 16'h002C; $display("d %0d", v inside {A + 8'd100});
    v = 16'h012C; $display("e %0d", v inside {[8'd200 + 8'd100 : 16'hFFFF]});
    v = 16'h012C; $display("f %0d", v == (8'd200 + 8'd100));
    $finish;
  end
endmodule
