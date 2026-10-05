module top;
  logic [71:0] v; int m;
  initial begin
    v = 72'h80_0000_0000_0000_0050; m = 9;
    case (v) inside
      72'h80_0000_0000_0000_00?0: m = 1;
      [72'h1:72'h3]: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    v = 72'h80_0000_0000_0000_0051; m = 9;
    case (v) inside
      72'h80_0000_0000_0000_00?0: m = 1;
      [72'h1:72'h3]: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    v = 72'h2; m = 9;
    case (v) inside
      72'h80_0000_0000_0000_00?0: m = 1;
      [72'h1:72'h3]: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    v = 72'h0; m = 9;
    case (v) inside
      72'h80_0000_0000_0000_00?0: m = 1;
      [72'h1:72'h3]: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    #10 $finish;
  end
endmodule
