// 002_traffic_fsm.sv — traffic-light FSM using a SystemVerilog typedef enum.
//
// Showcases the Phase-2 datatypes: the state register is an enum type, and the
// next-state logic switches on the enum labels. A clocked process advances the
// state; the testbench $display's each transition by name.
//
// Run:
//   vita examples/002_traffic_fsm.sv
// Produces: traffic_fsm.vcd

`timescale 1ns/1ns

module tb;
    // Enum state type. Labels auto-number from 0: GREEN=0, YELLOW=1, RED=2.
    typedef enum {GREEN, YELLOW, RED} light_t;

    reg     clk;
    reg     rst;
    light_t state;
    light_t next;
    integer cycles;

    // Combinational next-state: GREEN -> YELLOW -> RED -> GREEN ...
    always @* begin
        case (state)
            GREEN:   next = YELLOW;
            YELLOW:  next = RED;
            RED:     next = GREEN;
            default: next = GREEN;
        endcase
    end

    // Sequential state register with synchronous active-high reset.
    always @(posedge clk) begin
        if (rst)
            state <= GREEN;
        else
            state <= next;
    end

    // Render the current state label for $display.
    task show_state;
        case (state)
            GREEN:   $display("t=%0t  state=GREEN  (%0d)", $time, state);
            YELLOW:  $display("t=%0t  state=YELLOW (%0d)", $time, state);
            RED:     $display("t=%0t  state=RED    (%0d)", $time, state);
            default: $display("t=%0t  state=?      (%0d)", $time, state);
        endcase
    endtask

    // 10ns clock.
    initial clk = 1'b0;
    always #5 clk = ~clk;

    initial begin
        $dumpfile("traffic_fsm.vcd");
        $dumpvars(0, tb);

        rst = 1'b1;
        @(posedge clk);          // loads GREEN
        @(negedge clk);
        rst = 1'b0;

        // Run two full light cycles (6 transitions), printing after each edge.
        for (cycles = 0; cycles < 7; cycles = cycles + 1) begin
            #1 show_state;
            @(posedge clk);
        end

        $display("done after %0d samples", cycles);
        $finish;
    end
endmodule
